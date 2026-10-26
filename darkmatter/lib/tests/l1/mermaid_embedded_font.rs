//! The embedded Mermaid font on Darkmatter's browser page
//! (`HtmlOptions::embed_mermaid_font`, `DarkmatterPage::with_embed_mermaid_font`).
//!
//! The faces are whatever this host's font chain resolves, so assertions that
//! depend on the face (Inter or a host face) branch on the page's own
//! comments; biscuit-visualized's tests pin each case with injected fonts.

use std::collections::BTreeSet;

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::highlighting::ColorMode;
use darkmatter::markdown::output::{HtmlOptions, MermaidMode};
use darkmatter::markdown::render_tree::entrypoints::render_tree_html;
use regex::Regex;

const FLOWCHART: &str = "flowchart LR\n    A[Start here] --> B{Is it?}\n    B -->|Yes| C[Store]\n";
/// Class names are bold, so the page draws a second weight.
const CLASS: &str = "classDiagram\n    class Animal {\n        +String name\n        +eat() void\n    }\n";
const SEQUENCE: &str = "sequenceDiagram\n    Alice->>Bob: Hello Bob\n    Bob-->>Alice: Hi Alice\n";

fn document(sources: &[&str]) -> Markdown {
    let mut text = String::from("# Diagrams\n\n");
    for source in sources {
        text.push_str(&format!("```mermaid\n{source}```\n\nBetween diagrams.\n\n"));
    }
    text.into()
}

fn page(md: &Markdown, mode: MermaidMode, color_mode: ColorMode, embed: Option<bool>) -> String {
    let term = Terminal::new_optimistic(100);
    let mut page = DarkmatterPage::new(&term).with_mermaid_mode(mode).with_color_mode(color_mode);
    if let Some(embed) = embed {
        page = page.with_embed_mermaid_font(embed);
    }
    page.render_to_browser(md).expect("browser render")
}

/// The `(family, weight)` of every `@font-face` rule on the page.
fn font_face_rules(html: &str) -> Vec<(String, String)> {
    let rule = Regex::new(r"@font-face\{font-family:'([^']+)';font-style:normal;font-weight:(\d+);src:url\(data:font/ttf;base64,[A-Za-z0-9+/=]+\) format\(.truetype.\)\}").unwrap();
    rule.captures_iter(html).map(|caps| (caps[1].to_string(), caps[2].to_string())).collect()
}

/// The family each label's `font-family` leads with in the embedded SVGs.
fn label_lead_families(html: &str) -> BTreeSet<String> {
    let decoded = html.replace("&quot;", "\"").replace("&#39;", "'").replace("&apos;", "'");
    let lead = Regex::new(r#"font-family(?:=["]|\s*:\s*)'([^']+)'"#).unwrap();
    lead.captures_iter(&decoded).map(|caps| caps[1].to_string()).collect()
}

/// The embedded-font markup: the page comments, then the `<style>` holding
/// the rules.
fn font_markup(html: &str) -> regex::Captures<'_> {
    let markup = Regex::new(r"(?s)((?:<!-- mermaid font: [^\n]*? -->\n)+)<style>\n(.*?)</style>").unwrap();
    markup.captures(html).unwrap_or_else(|| panic!("no embedded-font markup:\n{html}"))
}

#[test]
fn the_opt_in_is_off_by_default() {
    assert!(!HtmlOptions::default().embed_mermaid_font);
    let md = document(&[FLOWCHART]);
    let default = page(&md, MermaidMode::Image, ColorMode::Dark, None);
    assert_eq!(default, page(&md, MermaidMode::Image, ColorMode::Dark, Some(false)));
    assert!(default.contains("<svg"), "control: a static SVG");
    assert!(!default.contains("@font-face"), "{default}");
    assert!(!default.contains("mermaid font"), "{default}");
}

/// Without static SVG the option changes nothing, byte for byte, on both
/// HTML entry points.
#[test]
fn without_static_svg_the_opt_in_changes_nothing() {
    let md = document(&[FLOWCHART, CLASS]);
    for mode in [MermaidMode::Off, MermaidMode::Text] {
        let without = page(&md, mode, ColorMode::Dark, Some(false));
        let with = page(&md, mode, ColorMode::Dark, Some(true));
        assert_eq!(with, without, "{mode:?}");
        assert!(!with.contains("@font-face"), "{mode:?}");

        let mut options = HtmlOptions::default();
        options.mermaid_mode = mode;
        let without = render_tree_html(&md, &options).expect("html").output;
        options.embed_mermaid_font = true;
        assert_eq!(render_tree_html(&md, &options).expect("html").output, without, "{mode:?}");
    }
}

/// AC-E4: several diagrams, in dark and light pages, share one `<style>`
/// with one rule per face and weight, before the first diagram.
#[test]
fn a_page_of_diagrams_embeds_one_rule_per_face_and_weight() {
    let md = document(&[FLOWCHART, CLASS, SEQUENCE]);
    for color_mode in [ColorMode::Dark, ColorMode::Light] {
        let html = page(&md, MermaidMode::Image, color_mode, Some(true));
        let rules = font_face_rules(&html);
        let distinct: BTreeSet<_> = rules.iter().cloned().collect();
        assert_eq!(rules.len(), distinct.len(), "{color_mode:?}: a face and weight twice: {rules:?}");
        assert!(rules.iter().any(|(_, weight)| weight == "400"), "{rules:?}");
        assert!(rules.iter().any(|(_, weight)| weight == "700"), "the bold class name: {rules:?}");
        assert_eq!(html.matches("@font-face").count(), rules.len(), "{color_mode:?}");

        // Every family a label leads with is declared, and only those.
        let declared: BTreeSet<String> = rules.iter().map(|(family, _)| family.clone()).collect();
        assert_eq!(label_lead_families(&html), declared, "{color_mode:?}");

        // One comment per family, immediately before the one `<style>`,
        // which comes before the first diagram.
        let markup = font_markup(&html);
        assert_eq!(markup[1].matches("<!-- mermaid font:").count(), declared.len());
        assert_eq!(html.matches("<!-- mermaid font:").count(), declared.len());
        assert!(markup.get(0).unwrap().start() < html.find("<svg").unwrap());
        assert!(!html.contains("darkmatter:mermaid-font"), "the placeholder is replaced");
        for family in &declared {
            assert!(markup[1].contains(&format!("\"{family}\" embedded")), "{family}: {}", &markup[1]);
        }
    }
}

/// AC-E15: Inter's OFL line sits immediately before Inter's first rule, and
/// a page embedding only host faces has none.
#[test]
fn the_ofl_line_accompanies_inter_only() {
    let html = page(&document(&[FLOWCHART, CLASS]), MermaidMode::Image, ColorMode::Dark, Some(true));
    let css = font_markup(&html)[2].to_string();
    let ofl = "/* Inter: Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter); SIL Open Font License 1.1 */";
    if font_face_rules(&html).iter().any(|(family, _)| family == "Inter") {
        let first_inter = css.find("@font-face{font-family:'Inter'").unwrap();
        assert_eq!(css[..first_inter].trim_end().rsplit('\n').next(), Some(ofl), "{css}");
        assert_eq!(css.matches("Open Font License").count(), 1);
    } else {
        assert!(!html.contains("Open Font License"), "{css}");
    }
}

/// AC-E14/E15 with Inter pinned: `MERMAID_FONT=Inter` makes every family
/// Inter, so the page has one comment and the OFL line precedes the first
/// rule. The variable is read once per process, when the font chain is
/// first built; nextest runs each test in its own process.
#[test]
fn mermaid_font_inter_embeds_inter_with_its_ofl_line() {
    // SAFETY: set before this process renders anything or starts a thread.
    unsafe { std::env::set_var("MERMAID_FONT", "Inter") };
    let html = page(&document(&[FLOWCHART, CLASS]), MermaidMode::Image, ColorMode::Dark, Some(true));
    let rules = font_face_rules(&html);
    assert!(!rules.is_empty() && rules.iter().all(|(family, _)| family == "Inter"), "{rules:?}");
    let markup = font_markup(&html);
    assert_eq!(markup[1].matches("<!-- mermaid font:").count(), 1, "{}", &markup[1]);
    assert!(markup[1].contains("\"Inter\" embedded"), "{}", &markup[1]);
    // The bundled Inter names `MERMAID_FONT` as the reason; a host with Inter
    // installed embeds that copy as the face the chain resolved.
    assert!(
        markup[1].contains("selected by MERMAID_FONT=Inter") || markup[1].contains("the face the font chain resolved"),
        "{}",
        &markup[1]
    );
    let ofl = "/* Inter: Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter); SIL Open Font License 1.1 */\n@font-face{font-family:'Inter'";
    assert!(markup[2].starts_with(ofl), "{}", &markup[2][..200.min(markup[2].len())]);
    assert_eq!(html.matches("Open Font License").count(), 1);
}

/// Both HTML entry points emit the same markup for the same page.
#[test]
fn the_html_entry_point_embeds_the_font_too() {
    let md = document(&[FLOWCHART, SEQUENCE]);
    let mut options = HtmlOptions::default();
    options.mermaid_mode = MermaidMode::Image;
    options.embed_mermaid_font = true;
    let html = render_tree_html(&md, &options).expect("html").output;
    assert!(!font_face_rules(&html).is_empty(), "{html}");
    assert!(!html.contains("darkmatter:mermaid-font"));
    let page_html = page(&md, MermaidMode::Image, ColorMode::Dark, Some(true));
    assert_eq!(font_markup(&html).get(0).unwrap().as_str(), font_markup(&page_html).get(0).unwrap().as_str());
}
