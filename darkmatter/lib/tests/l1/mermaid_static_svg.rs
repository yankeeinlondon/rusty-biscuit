//! Darkmatter's two browser Mermaid outputs, driven through the public page
//! path (`DarkmatterPage::render_to_browser`).
//!
//! - `MermaidMode::Off` keeps the interactive `<pre class="mermaid">` source
//!   and embeds no SVG.
//! - `MermaidMode::Image` (`BrowserMermaidMode::StaticSvg`) embeds
//!   biscuit-visualized's "HTML static" SVG after the sanitizer: labels, the
//!   measured-face font lead, label overflow, and drop shadows all survive.

use biscuit_terminal::terminal::Terminal;
use biscuit_visualized::components::mermaid::{MermaidDiagram, MermaidTarget};
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::MermaidMode;
use regex::Regex;

/// A flowchart (labels, drop shadows) and its label texts.
const FLOWCHART: &str = "flowchart LR\n    A[Start here] --> B{Is it?}\n    B -->|Yes| C[Store]\n";
const FLOWCHART_LABELS: [&str; 4] = ["Start here", "Is it?", "Yes", "Store"];

/// A class diagram: Merman clips its member labels with `overflow:hidden` on
/// nested `<svg>` viewports, which the HTML-static target opens.
const CLASS: &str = "classDiagram\n    class Animal {\n        +String name\n        +eat() void\n    }\n";

fn page_html(source: &str, mode: MermaidMode) -> String {
    let term = Terminal::new_optimistic(100);
    let md: Markdown = format!("```mermaid\n{source}```\n").into();
    DarkmatterPage::new(&term)
        .with_mermaid_mode(mode)
        .render_to_browser(&md)
        .expect("browser render")
}

/// The embedded `<svg>…</svg>`, so page CSS outside the diagram is not read.
fn embedded_svg(html: &str) -> &str {
    let start = html.find("<svg").expect("an embedded svg");
    let end = html.rfind("</svg>").expect("a closed svg") + "</svg>".len();
    &html[start..end]
}

/// Every `font-family` value in `svg`, from attributes, inline styles, and the
/// `<style>` element, with `&quot;` decoded so quoting does not affect equality.
fn font_families(svg: &str) -> Vec<String> {
    let decoded = svg.replace("&quot;", "\"").replace("&#39;", "'").replace("&apos;", "'");
    let attribute = Regex::new(r#"font-family="([^"]*)""#).unwrap();
    let declaration = Regex::new(r#"font-family\s*:\s*([^;}"]*(?:"[^"]*"[^;}"]*)*)"#).unwrap();
    attribute
        .captures_iter(&decoded)
        .chain(declaration.captures_iter(&decoded))
        .map(|caps| caps[1].trim().trim_end_matches("!important").trim().to_string())
        .collect()
}

#[test]
fn mermaid_off_keeps_the_interactive_source_and_embeds_no_svg() {
    let html = page_html(FLOWCHART, MermaidMode::Off);
    assert!(html.contains(r#"<pre class="mermaid""#), "Off must emit the interactive source:\n{html}");
    assert!(!html.contains("<svg"), "Off must embed no SVG:\n{html}");
}

#[test]
fn mermaid_image_embeds_a_static_svg_whose_labels_and_shadows_survive() {
    let html = page_html(FLOWCHART, MermaidMode::Image);
    assert!(html.contains("<svg"), "Image must embed the static SVG:\n{html}");
    assert!(!html.contains(r#"<pre class="mermaid""#), "Image must not emit the interactive source");
    for label in FLOWCHART_LABELS {
        assert!(html.contains(label), "label {label:?} must survive the sanitizer:\n{html}");
    }
    let unsanitized = MermaidDiagram::new(FLOWCHART)
        .with_target(MermaidTarget::HtmlStatic)
        .render_to_svg()
        .expect("html-static svg");
    let shadows = unsanitized.matches("<feDropShadow").count();
    assert!(shadows > 0, "the flowchart fixture must carry Merman drop shadows");
    assert_eq!(
        html.matches("<feDropShadow").count(),
        shadows,
        "every Merman drop shadow must survive the sanitizer"
    );
    assert!(html.contains("<style"), "Merman's scoped <style> must survive");
}

/// Each label font in the embedded SVG is the measured face followed by the
/// original stack: the page's values equal the raster-safe values with one
/// quoted family prepended, and match the HTML-static render exactly.
#[test]
fn mermaid_image_leads_every_label_font_with_the_measured_face() {
    let html = page_html(FLOWCHART, MermaidMode::Image);
    let embedded = font_families(embedded_svg(&html));
    let html_static = font_families(
        &MermaidDiagram::new(FLOWCHART)
            .with_target(MermaidTarget::HtmlStatic)
            .render_to_svg()
            .expect("html-static svg"),
    );
    let raster_safe = font_families(&MermaidDiagram::new(FLOWCHART).render_to_svg().expect("raster-safe svg"));

    assert!(!embedded.is_empty(), "the embedded SVG must declare label fonts");
    assert_eq!(embedded, html_static, "the sanitizer must keep every font-family value");
    assert_eq!(embedded.len(), raster_safe.len());
    let lead = Regex::new(r"^'([^']+)',(.+)$").unwrap();
    for (html_value, original) in embedded.iter().zip(&raster_safe) {
        let caps = lead
            .captures(html_value)
            .unwrap_or_else(|| panic!("{html_value:?} must lead with a quoted measured face"));
        assert_eq!(&caps[2], original, "the original stack must follow the measured face");
    }
}

#[test]
fn mermaid_image_lets_clipped_labels_overflow() {
    let html = page_html(CLASS, MermaidMode::Image);
    let raster_safe = MermaidDiagram::new(CLASS).render_to_svg().expect("raster-safe svg");
    assert!(
        raster_safe.contains("overflow:hidden") || raster_safe.contains("overflow: hidden"),
        "control: Merman clips class labels in the raster-safe SVG"
    );
    let visible = Regex::new(r#"overflow(?:="visible"|\s*:\s*visible)"#).unwrap();
    assert!(visible.is_match(&html), "the embedded SVG must let labels overflow:\n{html}");
    assert!(html.contains("Animal"), "the class label must survive");
}
