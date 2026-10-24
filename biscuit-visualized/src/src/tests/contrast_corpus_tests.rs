//! AC-T4: every text element in the golden corpus, the derived-palette
//! goldens included, meets 4.5:1 against the fill behind it, whatever its
//! size (D18, D45, R-V3).
//!
//! The oracle is independent of the production contrast pass
//! (`mermaid/contrast.rs`), which finds the fill behind a label from inline
//! paint only. Here usvg resolves the stored golden SVG (stylesheets, `<use>`
//! placements, transforms) and gives each text its color and its glyphs'
//! box; the same SVG with every `<text>` removed is rasterized onto the
//! palette background, and the fill behind a text is the most frequent pixel
//! inside its box there. Limits: a text's box is judged as a whole, so a
//! label straddling two fills is judged on the larger one; a fill smaller
//! than the glyphs (a thin edge under a label) loses to what surrounds it;
//! text stroke (halo) is ignored.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use resvg::tiny_skia::{Color, Pixmap};
use resvg::usvg;

use super::golden_tests::{DERIVED_GOLDENS, GOLDEN_FIXTURES, GOLDEN_SOURCES, derived_palettes, golden_svg_text};
use crate::fonts::FontChain;
use crate::mermaid::{MermaidTheme, Palette, Rgb};
use crate::raster::usvg_options;

const TEXT_CONTRAST: f64 = 4.5;

/// Rasterization scale of the text-free SVG. 1× finds the same fills as 2×
/// over the corpus at a third of the time.
const SCALE: f32 = 1.0;

const FIXED_PALETTES: [(&str, MermaidTheme); 4] = [
    ("dark", MermaidTheme::Dark),
    ("default", MermaidTheme::Default),
    ("forest", MermaidTheme::Forest),
    ("neutral", MermaidTheme::Neutral),
];

/// A known failure: every text of `fixture` × `palette` on `behind` (or only
/// `text`) with the reason. Each entry must still match a failure, so it is
/// removed when its cause is fixed.
struct Allowed {
    fixture: &'static str,
    palette: &'static str,
    text: Option<&'static str>,
    behind: &'static str,
    #[allow(dead_code, reason = "read in the source, not by the test")]
    reason: &'static str,
}

impl Allowed {
    fn matches(&self, fixture: &str, palette: &str, failure: &Failure) -> bool {
        self.fixture == fixture
            && self.palette == palette
            && self.behind == failure.behind
            && self.text.is_none_or(|text| text == failure.text)
    }
}

const ALLOWED: &[Allowed] = &[
    Allowed {
        fixture: "c4",
        palette: "dark",
        text: None,
        behind: "#808080",
        reason: "the author's `UpdateElementStyle(customerA, $fontColor=\"red\", $bgColor=\"grey\")`: their text color on their own fill",
    },
    Allowed {
        fixture: "c4",
        palette: "default",
        text: None,
        behind: "#808080",
        reason: "the author's `UpdateElementStyle(customerA, $fontColor=\"red\", $bgColor=\"grey\")`: their text color on their own fill",
    },
    Allowed {
        fixture: "c4",
        palette: "forest",
        text: None,
        behind: "#808080",
        reason: "the author's `UpdateElementStyle(customerA, $fontColor=\"red\", $bgColor=\"grey\")`: their text color on their own fill",
    },
    Allowed {
        fixture: "c4",
        palette: "neutral",
        text: None,
        behind: "#808080",
        reason: "the author's `UpdateElementStyle(customerA, $fontColor=\"red\", $bgColor=\"grey\")`: their text color on their own fill",
    },
    Allowed {
        fixture: "c4",
        palette: "dark",
        text: Some("Uses"),
        behind: "#333333",
        reason: "the author's `UpdateRelStyle(..., $textColor=\"blue\")` on the palette background",
    },
    Allowed {
        fixture: "c4",
        palette: "dark",
        text: Some("Sends e-mails to"),
        behind: "#333333",
        reason: "the author's `UpdateRelStyle(..., $textColor=\"red\")` on the palette background",
    },
    Allowed {
        fixture: "c4",
        palette: "default",
        text: Some("Sends e-mails to"),
        behind: "#ffffff",
        reason: "the author's `UpdateRelStyle(..., $textColor=\"red\")` on the palette background",
    },
    Allowed {
        fixture: "c4",
        palette: "forest",
        text: Some("Sends e-mails to"),
        behind: "#ffffff",
        reason: "the author's `UpdateRelStyle(..., $textColor=\"red\")` on the palette background",
    },
    Allowed {
        fixture: "c4",
        palette: "neutral",
        text: Some("Sends e-mails to"),
        behind: "#ffffff",
        reason: "the author's `UpdateRelStyle(..., $textColor=\"red\")` on the palette background",
    },
    Allowed {
        fixture: "venn",
        palette: "dark",
        text: Some("AB"),
        behind: "#464434",
        reason: "the author's `style A,B color:#333` on the dark palette's intersection tint: an author's text color is kept on any fill",
    },
    Allowed {
        fixture: "venn",
        palette: "dark",
        text: Some("React"),
        behind: "#483939",
        reason: "the author's `style A1 color:red` inside their `style A fill:#ff6b6b` set",
    },
    Allowed {
        fixture: "venn",
        palette: "default",
        text: Some("React"),
        behind: "#fff0f0",
        reason: "the author's `style A1 color:red` inside their `style A fill:#ff6b6b` set",
    },
    Allowed {
        fixture: "venn",
        palette: "forest",
        text: Some("React"),
        behind: "#fff0f0",
        reason: "the author's `style A1 color:red` inside their `style A fill:#ff6b6b` set",
    },
    Allowed {
        fixture: "venn",
        palette: "neutral",
        text: Some("React"),
        behind: "#fff0f0",
        reason: "the author's `style A1 color:red` inside their `style A fill:#ff6b6b` set",
    },
];

fn golden_dir() -> PathBuf {
    biscuit_test_harness::manifest_dir!().join("src/tests/goldens/svg")
}

/// The goldens' font chain (bundled Inter only), as `golden_tests` renders.
fn chain() -> &'static Arc<FontChain> {
    static CHAIN: OnceLock<Arc<FontChain>> = OnceLock::new();
    CHAIN.get_or_init(|| Arc::new(FontChain::injected(&[], &[], Some("Inter"))))
}

/// The SVG of an `insta` snapshot file: everything after its `---` header.
fn snapshot_body(snapshot: &str) -> &str {
    let header = snapshot.strip_prefix("---\n").expect("an insta header");
    let end = header.find("\n---\n").expect("the end of the insta header");
    header[end + 5..].trim_end()
}

/// `svg` with every `<text>` element removed.
fn strip_text(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(at) = find_text_open(rest) {
        out.push_str(&rest[..at]);
        let from = &rest[at..];
        let tag_end = from.find('>').expect("a closed <text> tag");
        rest = if from[..tag_end].ends_with('/') {
            &from[tag_end + 1..]
        } else {
            let close = from.find("</text>").expect("a </text>");
            &from[close + "</text>".len()..]
        };
    }
    out.push_str(rest);
    out
}

fn find_text_open(svg: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(at) = svg[offset..].find("<text") {
        let start = offset + at;
        let next = svg.as_bytes().get(start + "<text".len()).copied();
        if matches!(next, Some(b' ' | b'>' | b'/' | b'\n' | b'\t' | b'\r')) {
            return Some(start);
        }
        offset = start + 1;
    }
    None
}

/// The fill of one text span, as drawn.
#[derive(Debug, Clone, PartialEq)]
enum TextFill {
    /// A solid color and its effective opacity (fill opacity times every
    /// enclosing group's opacity).
    Solid(Rgb, f32),
    /// A gradient or pattern: not measurable as one color.
    NonSolid,
}

/// A text node of the parsed tree: its content, its spans' distinct fills,
/// and its glyphs' box in canvas units.
#[derive(Debug)]
struct TextItem {
    content: String,
    fills: Vec<TextFill>,
    bbox: usvg::Rect,
}

fn collect_texts(group: &usvg::Group, opacity: f32, out: &mut Vec<TextItem>) {
    for node in group.children() {
        match node {
            usvg::Node::Group(child) => collect_texts(child, opacity * child.opacity().get(), out),
            usvg::Node::Text(text) => {
                let mut fills = Vec::new();
                let mut content = Vec::new();
                for chunk in text.chunks() {
                    for span in chunk.spans() {
                        let piece = &chunk.text()[span.start()..span.end()];
                        if piece.trim().is_empty() || !span.is_visible() {
                            continue;
                        }
                        content.push(piece.trim().to_owned());
                        let Some(fill) = span.fill() else { continue };
                        let fill = match fill.paint() {
                            usvg::Paint::Color(color) => TextFill::Solid(
                                Rgb::new(color.red, color.green, color.blue),
                                opacity * fill.opacity().get(),
                            ),
                            _ => TextFill::NonSolid,
                        };
                        if !fills.contains(&fill) {
                            fills.push(fill);
                        }
                    }
                }
                if fills.is_empty() {
                    continue;
                }
                // The glyph outlines' box (ink), not the font's
                // ascent-to-descent line box, which can overhang a tight
                // label background. The flattened group's box is in the
                // text's own coordinates.
                let bbox = text
                    .flattened()
                    .bounding_box()
                    .transform(text.abs_transform())
                    .unwrap_or_else(|| text.abs_bounding_box());
                out.push(TextItem { content: content.join(" "), fills, bbox });
            }
            _ => {}
        }
    }
}

/// The most frequent color in `pixmap` inside `bbox` (canvas units, scaled
/// by `scale`), or `None` when the box covers no pixel.
fn dominant_color(pixmap: &Pixmap, bbox: usvg::Rect, scale: f32) -> Option<Rgb> {
    let clamp_x = |value: f32| (value * scale).clamp(0.0, pixmap.width() as f32) as u32;
    let clamp_y = |value: f32| (value * scale).clamp(0.0, pixmap.height() as f32) as u32;
    let (left, right) = (clamp_x(bbox.left().floor()), clamp_x(bbox.right().ceil()));
    let (top, bottom) = (clamp_y(bbox.top().floor()), clamp_y(bbox.bottom().ceil()));
    let mut counts: HashMap<Rgb, u32> = HashMap::new();
    for y in top..bottom {
        for x in left..right {
            let pixel = pixmap.pixel(x, y).expect("in bounds").demultiply();
            // The canvas starts filled with the opaque palette background, so
            // every pixel is opaque.
            *counts.entry(Rgb::new(pixel.red(), pixel.green(), pixel.blue())).or_default() += 1;
        }
    }
    counts.into_iter().max_by_key(|(color, count)| (*count, color.r, color.g, color.b)).map(|(color, _)| color)
}

/// One text whose contrast is below the floor (or cannot be measured).
#[derive(Debug)]
struct Failure {
    text: String,
    color: String,
    behind: String,
    ratio: f64,
}

/// Every failing text of `svg`, drawn on `background`.
fn contrast_failures(svg: &str, background: Rgb) -> Vec<Failure> {
    let options = usvg_options(chain());
    let tree = usvg::Tree::from_str(svg, &options).expect("the golden parses");
    let mut texts = Vec::new();
    collect_texts(tree.root(), 1.0, &mut texts);

    let bare = usvg::Tree::from_str(&strip_text(svg), &options).expect("the text-free golden parses");
    let size = bare.size();
    let mut pixmap = Pixmap::new(
        ((size.width() * SCALE).round() as u32).max(1),
        ((size.height() * SCALE).round() as u32).max(1),
    )
    .expect("a canvas");
    pixmap.fill(Color::from_rgba8(background.r, background.g, background.b, 255));
    resvg::render(&bare, usvg::Transform::from_scale(SCALE, SCALE), &mut pixmap.as_mut());

    let mut failures = Vec::new();
    for item in texts {
        let Some(behind) = dominant_color(&pixmap, item.bbox, SCALE) else {
            failures.push(Failure {
                text: item.content,
                color: "-".into(),
                behind: "outside the canvas".into(),
                ratio: 0.0,
            });
            continue;
        };
        for fill in &item.fills {
            let (color, ratio) = match *fill {
                TextFill::Solid(color, alpha) => {
                    let drawn = color.over(behind, f64::from(alpha));
                    let label = if alpha < 1.0 { format!("{color} at {alpha:.2} = {drawn}") } else { color.to_string() };
                    (label, drawn.contrast(behind))
                }
                TextFill::NonSolid => ("non-solid fill".into(), 0.0),
            };
            if ratio < TEXT_CONTRAST {
                failures.push(Failure {
                    text: item.content.clone(),
                    color,
                    behind: behind.to_string(),
                    ratio,
                });
            }
        }
    }
    failures
}

/// The palettes a fixture's goldens were rendered under, by golden name.
fn palettes_for(fixture: &str) -> Vec<(&'static str, Palette)> {
    let mut palettes: Vec<(&str, Palette)> =
        FIXED_PALETTES.iter().map(|(name, theme)| (*name, Palette::fixed(*theme))).collect();
    for (name, palette) in derived_palettes() {
        if DERIVED_GOLDENS.contains(&(fixture, name)) {
            palettes.push((name, palette));
        }
    }
    palettes
}

fn assert_family_contrast(fixture: &str) {
    let mut unexpected = Vec::new();
    let mut allowed_seen = Vec::new();
    for (palette_name, palette) in palettes_for(fixture) {
        let path = golden_dir().join(format!("{fixture}_{palette_name}.snap"));
        let snapshot = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        for failure in contrast_failures(snapshot_body(&snapshot), palette.background()) {
            match ALLOWED.iter().position(|allowed| allowed.matches(fixture, palette_name, &failure)) {
                Some(index) => allowed_seen.push(index),
                None => unexpected.push(format!(
                    "{fixture} × {palette_name}: {:?} {} on {} = {:.2}:1",
                    failure.text, failure.color, failure.behind, failure.ratio
                )),
            }
        }
    }
    let stale: Vec<String> = ALLOWED
        .iter()
        .enumerate()
        .filter(|(index, allowed)| allowed.fixture == fixture && !allowed_seen.contains(index))
        .map(|(_, allowed)| format!("{} × {}: {:?} on {}", allowed.fixture, allowed.palette, allowed.text, allowed.behind))
        .collect();
    assert!(stale.is_empty(), "allow-list entries that no longer fail:\n{}", stale.join("\n"));
    assert!(
        unexpected.is_empty(),
        "text below {TEXT_CONTRAST}:1 against the fill behind it:\n{}",
        unexpected.join("\n")
    );
}

macro_rules! corpus_contrast {
    ($($test:ident => $fixture:literal),+ $(,)?) => {
        /// The fixtures with a contrast test.
        const CHECKED_FIXTURES: &[&str] = &[$($fixture),+];

        $(
            #[test]
            fn $test() {
                assert_family_contrast($fixture);
            }
        )+
    };
}

corpus_contrast! {
    flowchart_text_is_readable => "flowchart",
    swimlane_text_is_readable => "swimlane",
    mindmap_text_is_readable => "mindmap",
    architecture_text_is_readable => "architecture",
    sequence_text_is_readable => "sequence",
    c4_text_is_readable => "c4",
    kanban_text_is_readable => "kanban",
    class_text_is_readable => "class",
    er_text_is_readable => "er",
    gantt_text_is_readable => "gantt",
    info_text_is_readable => "info",
    pie_text_is_readable => "pie",
    requirement_text_is_readable => "requirement",
    timeline_text_is_readable => "timeline",
    gitgraph_text_is_readable => "gitgraph",
    state_text_is_readable => "state",
    journey_text_is_readable => "journey",
    quadrant_chart_text_is_readable => "quadrantchart",
    sankey_text_is_readable => "sankey",
    packet_text_is_readable => "packet",
    xychart_text_is_readable => "xychart",
    block_text_is_readable => "block",
    event_modeling_text_is_readable => "eventmodeling",
    tree_view_text_is_readable => "treeView",
    radar_text_is_readable => "radar",
    ishikawa_text_is_readable => "ishikawa",
    treemap_text_is_readable => "treemap",
    railroad_text_is_readable => "railroad",
    venn_text_is_readable => "venn",
    wardley_text_is_readable => "wardley",
    cynefin_text_is_readable => "cynefin",
    usecase_text_is_readable => "usecase",
    agentflow_text_is_readable => "agentflow",
}

/// Every stored SVG golden is checked by some test above, and every golden
/// family has one.
#[test]
fn every_svg_golden_has_a_contrast_check() {
    let mut unchecked = Vec::new();
    for entry in std::fs::read_dir(golden_dir()).expect("the SVG golden directory") {
        let name = entry.expect("a directory entry").file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".snap") else { continue };
        let (fixture, palette) = stem.split_once('_').expect("<fixture>_<palette>");
        let covered = CHECKED_FIXTURES.contains(&fixture)
            && palettes_for(fixture).iter().any(|(name, _)| *name == palette);
        if !covered {
            unchecked.push(name);
        }
    }
    assert!(unchecked.is_empty(), "goldens without a contrast check: {unchecked:?}");
    let mut checked = CHECKED_FIXTURES.to_vec();
    checked.sort_unstable();
    let mut goldens = GOLDEN_FIXTURES.to_vec();
    goldens.sort_unstable();
    assert_eq!(checked, goldens);
}

#[test]
fn strip_text_removes_every_text_element_and_nothing_else() {
    let svg = r#"<svg><rect/><text x="1">a<tspan>b</tspan></text><g><text/></g><textPath/><text
y="2">c</text></svg>"#;
    assert_eq!(strip_text(svg), "<svg><rect/><g></g><textPath/></svg>");
}

#[test]
fn snapshot_body_skips_the_insta_header() {
    assert_eq!(snapshot_body("---\nsource: x.rs\n---\n<svg/>\n"), "<svg/>");
}

#[test]
fn dominant_color_is_the_most_frequent_pixel_in_the_box() {
    let mut pixmap = Pixmap::new(10, 10).unwrap();
    pixmap.fill(Color::from_rgba8(255, 255, 255, 255));
    let mut paint = resvg::tiny_skia::Paint::default();
    paint.set_color_rgba8(0, 0, 255, 255);
    let rect = resvg::tiny_skia::Rect::from_xywh(0.0, 0.0, 6.0, 10.0).unwrap();
    pixmap.fill_rect(rect, &paint, resvg::tiny_skia::Transform::identity(), None);

    let whole = usvg::Rect::from_xywh(0.0, 0.0, 10.0, 10.0).unwrap();
    assert_eq!(dominant_color(&pixmap, whole, 1.0), Some(Rgb::new(0, 0, 255)));
    let right = usvg::Rect::from_xywh(2.5, 0.0, 2.5, 5.0).unwrap();
    assert_eq!(dominant_color(&pixmap, right, 2.0), Some(Rgb::WHITE));
    let outside = usvg::Rect::from_xywh(20.0, 20.0, 5.0, 5.0).unwrap();
    assert_eq!(dominant_color(&pixmap, outside, 1.0), None);
}

/// A label painted through a stylesheet on a stylesheet-painted box, placed
/// with `<use>` under a `viewBox` offset, is judged on that box: the oracle
/// sees what the production pass calls "unknown".
#[test]
fn the_oracle_fails_a_low_contrast_label_and_passes_a_readable_one() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="200" height="100" viewBox="0 10 200 100">
<style>.box{fill:#777777}.dim{fill:#888888}</style>
<rect x="0" y="10" width="200" height="100" fill="#ffffff"/>
<rect class="box" x="10" y="20" width="80" height="40"/>
<rect x="110" y="20" width="80" height="40" fill="#000000"/>
<defs><g id="label"><text class="dim" x="20" y="45" font-family="Inter" font-size="16">Dim</text></g></defs>
<use href="#label"/>
<text x="120" y="45" fill="#ffffff" font-family="Inter" font-size="16">Bright</text>
<text x="20" y="95" fill="#000000" fill-opacity="0.3" font-family="Inter" font-size="16">Faint</text>
<text x="120" y="95" font-family="Inter" font-size="16"> </text>
</svg>"##;
    let failures = contrast_failures(svg, Rgb::WHITE);
    let summary: Vec<(&str, &str, &str)> =
        failures.iter().map(|f| (f.text.as_str(), f.color.as_str(), f.behind.as_str())).collect();
    assert_eq!(
        summary,
        [("Dim", "#888888", "#777777"), ("Faint", "#000000 at 0.30 = #b2b2b2", "#ffffff")],
        "{failures:?}"
    );
}

// ---------------------------------------------------------------------------
// Author colors with no compiled design (R2-F1)
// ---------------------------------------------------------------------------

/// The one author edit of the sweep: a white node fill. An author color
/// displaces the compiled design, so Merman paints every surface from its
/// stylesheet and the library's text must still read on the author's fill.
const WHITE_PRIMARY: &str = "%%{init: {'themeVariables': {'primaryColor': '#ffffff'}}}%%\n";

/// `source` with [`WHITE_PRIMARY`] after its YAML frontmatter, if any, so
/// the frontmatter's own configuration is kept.
fn with_white_primary(source: &str) -> String {
    match source.strip_prefix("---\n") {
        Some(rest) => {
            let end = rest.find("\n---\n").expect("the end of the frontmatter") + "\n---\n".len();
            format!("---\n{}{WHITE_PRIMARY}{}", &rest[..end], &rest[end..])
        }
        None => format!("{WHITE_PRIMARY}{source}"),
    }
}

/// A text color the fixture's author wrote, which the contrast pass keeps
/// even where it fails. Each entry must still match a failure.
struct AuthorText {
    fixture: &'static str,
    color: &'static str,
    #[allow(dead_code, reason = "read in the source, not by the test")]
    reason: &'static str,
}

const AUTHOR_TEXT: &[AuthorText] = &[
    AuthorText {
        fixture: "c4",
        color: "#ff0000",
        reason: "the author's `$fontColor=\"red\"` on their grey element and `$textColor=\"red\"` on a relationship",
    },
    AuthorText { fixture: "c4", color: "#0000ff", reason: "the author's `$textColor=\"blue\"` on a relationship" },
    AuthorText { fixture: "venn", color: "#ff0000", reason: "the author's `style A1 color:red`" },
    AuthorText { fixture: "venn", color: "#333333", reason: "the author's `style A,B color:#333`" },
];

/// Every family rendered `Dark` through the public render path with a white
/// `primaryColor`: every text the library colored reads on what is painted
/// behind it, by the same independent oracle as the corpus.
fn assert_white_primary_contrast(fixture: &str) {
    let (_, source) = GOLDEN_SOURCES
        .iter()
        .find(|(name, _)| *name == fixture)
        .unwrap_or_else(|| panic!("no fixture {fixture}"));
    let palette = Palette::fixed(MermaidTheme::Dark);
    let svg = golden_svg_text(&with_white_primary(source), palette);
    let mut unexpected = Vec::new();
    let mut author_seen = Vec::new();
    for failure in contrast_failures(&svg, palette.background()) {
        match AUTHOR_TEXT.iter().position(|author| author.fixture == fixture && author.color == failure.color) {
            Some(index) => author_seen.push(index),
            None => unexpected.push(format!(
                "{fixture} × white primaryColor: {:?} {} on {} = {:.2}:1",
                failure.text, failure.color, failure.behind, failure.ratio
            )),
        }
    }
    let stale: Vec<&str> = AUTHOR_TEXT
        .iter()
        .enumerate()
        .filter(|(index, author)| author.fixture == fixture && !author_seen.contains(index))
        .map(|(_, author)| author.color)
        .collect();
    assert!(stale.is_empty(), "{fixture}: author text colors that no longer fail: {stale:?}");
    assert!(
        unexpected.is_empty(),
        "text below {TEXT_CONTRAST}:1 against the fill behind it:\n{}",
        unexpected.join("\n")
    );
}

macro_rules! white_primary_contrast {
    ($($test:ident => $fixture:literal),+ $(,)?) => {
        /// The fixtures with a white-`primaryColor` contrast test.
        const WHITE_PRIMARY_FIXTURES: &[&str] = &[$($fixture),+];

        $(
            #[test]
            fn $test() {
                assert_white_primary_contrast($fixture);
            }
        )+
    };
}

white_primary_contrast! {
    flowchart_text_reads_on_a_white_primary_color => "flowchart",
    swimlane_text_reads_on_a_white_primary_color => "swimlane",
    mindmap_text_reads_on_a_white_primary_color => "mindmap",
    architecture_text_reads_on_a_white_primary_color => "architecture",
    sequence_text_reads_on_a_white_primary_color => "sequence",
    c4_text_reads_on_a_white_primary_color => "c4",
    kanban_text_reads_on_a_white_primary_color => "kanban",
    class_text_reads_on_a_white_primary_color => "class",
    er_text_reads_on_a_white_primary_color => "er",
    gantt_text_reads_on_a_white_primary_color => "gantt",
    info_text_reads_on_a_white_primary_color => "info",
    pie_text_reads_on_a_white_primary_color => "pie",
    requirement_text_reads_on_a_white_primary_color => "requirement",
    timeline_text_reads_on_a_white_primary_color => "timeline",
    gitgraph_text_reads_on_a_white_primary_color => "gitgraph",
    state_text_reads_on_a_white_primary_color => "state",
    journey_text_reads_on_a_white_primary_color => "journey",
    quadrant_chart_text_reads_on_a_white_primary_color => "quadrantchart",
    sankey_text_reads_on_a_white_primary_color => "sankey",
    packet_text_reads_on_a_white_primary_color => "packet",
    xychart_text_reads_on_a_white_primary_color => "xychart",
    block_text_reads_on_a_white_primary_color => "block",
    event_modeling_text_reads_on_a_white_primary_color => "eventmodeling",
    tree_view_text_reads_on_a_white_primary_color => "treeView",
    radar_text_reads_on_a_white_primary_color => "radar",
    ishikawa_text_reads_on_a_white_primary_color => "ishikawa",
    treemap_text_reads_on_a_white_primary_color => "treemap",
    railroad_text_reads_on_a_white_primary_color => "railroad",
    venn_text_reads_on_a_white_primary_color => "venn",
    wardley_text_reads_on_a_white_primary_color => "wardley",
    cynefin_text_reads_on_a_white_primary_color => "cynefin",
    usecase_text_reads_on_a_white_primary_color => "usecase",
    agentflow_text_reads_on_a_white_primary_color => "agentflow",
}

/// Every family fixture on disk has a white-`primaryColor` test.
#[test]
fn every_family_fixture_has_a_white_primary_test() {
    let fixtures_dir = biscuit_test_harness::manifest_dir!().join("src/tests/fixtures/merman/families");
    let mut on_disk: Vec<String> = std::fs::read_dir(&fixtures_dir)
        .expect("the family fixture directory")
        .filter_map(|entry| {
            let name = entry.expect("a directory entry").file_name().to_string_lossy().into_owned();
            name.strip_suffix(".mmd").map(str::to_owned)
        })
        .collect();
    on_disk.sort_unstable();
    let mut tested: Vec<String> = WHITE_PRIMARY_FIXTURES.iter().map(|name| (*name).to_owned()).collect();
    tested.sort_unstable();
    assert_eq!(tested, on_disk);
}

#[test]
fn the_white_primary_directive_follows_any_frontmatter() {
    assert_eq!(with_white_primary("pie\n  \"A\": 1\n"), format!("{WHITE_PRIMARY}pie\n  \"A\": 1\n"));
    assert_eq!(
        with_white_primary("---\ntitle: T\n---\npie\n"),
        format!("---\ntitle: T\n---\n{WHITE_PRIMARY}pie\n")
    );
}
