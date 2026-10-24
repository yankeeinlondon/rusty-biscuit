//! The code points and weights a static SVG draws, per family: what the
//! embedded font must carry (Rule E, AC-E3's input).
//!
//! Hand-written SVGs pin each cascade rule; real Merman HTML-static output is
//! checked against usvg's own resolution of every text span (family, weight,
//! text), an oracle independent of the collector's cascade.

use std::collections::{BTreeMap, BTreeSet};

use resvg::usvg::{self, Node};

use super::postprocess_tests::{CLASS, FLOWCHART, chain, render, tree};
use crate::fonts::LabelMeasurer;
use crate::mermaid::embed::glyphs::{UsedGlyphs, WeightGlyphs, collect_used_glyphs, first_family};
use crate::mermaid::postprocess::{css_quoted, html_static, postprocess};

const SEQUENCE: &str = "sequenceDiagram\n  Alice->>Bob: Hi there\n  Note right of Bob: thinks";

fn svg(body: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" font-family="Inter, sans-serif">{body}</svg>"#)
}

fn chars(text: &str) -> BTreeSet<char> {
    text.chars().collect()
}

fn collect(svg: &str) -> UsedGlyphs {
    collect_used_glyphs(svg).expect("the SVG parses")
}

fn only_inter(used: &UsedGlyphs) -> &WeightGlyphs {
    assert_eq!(used.families.keys().collect::<Vec<_>>(), ["Inter"], "{used:?}");
    &used.families["Inter"]
}

/// The final HTML static SVG of `source`, as the opt-in serves it.
fn html_static_svg(source: &str) -> String {
    let raster = postprocess(&render(source, "default")).into_owned();
    html_static(&raster, &LabelMeasurer::new(chain())).into_owned()
}

/// What usvg draws in `svg`: per first family, the non-space characters of
/// every span, split at weight 600.
fn usvg_drawn(svg: &str) -> BTreeMap<String, WeightGlyphs> {
    fn walk(group: &usvg::Group, into: &mut BTreeMap<String, WeightGlyphs>) {
        for node in group.children() {
            match node {
                Node::Group(child) => walk(child, into),
                Node::Text(text) => {
                    for chunk in text.chunks() {
                        for span in chunk.spans() {
                            let family = match &span.font().families()[0] {
                                usvg::FontFamily::Named(name) => name.clone(),
                                generic => generic.to_string(),
                            };
                            let glyphs = into.entry(family).or_default();
                            let set = if span.font().weight() >= 600 { &mut glyphs.bold } else { &mut glyphs.regular };
                            set.extend(chunk.text()[span.start()..span.end()].chars().filter(|c| *c != ' '));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut drawn = BTreeMap::new();
    walk(tree(svg).root(), &mut drawn);
    drawn
}

fn without_spaces(glyphs: &WeightGlyphs) -> WeightGlyphs {
    let strip = |set: &BTreeSet<char>| set.iter().copied().filter(|c| *c != ' ').collect();
    WeightGlyphs {
        regular: strip(&glyphs.regular),
        bold: strip(&glyphs.bold),
    }
}

#[test]
fn real_merman_output_matches_what_usvg_draws() {
    for source in [FLOWCHART, CLASS, SEQUENCE] {
        let svg = html_static_svg(source);
        let used = collect(&svg);
        let expected = usvg_drawn(&svg);
        let actual: BTreeMap<String, WeightGlyphs> = used
            .families
            .iter()
            .map(|(family, glyphs)| (family.clone(), without_spaces(glyphs)))
            .collect();
        assert_eq!(actual, expected, "{source}");
        assert!(used.families.contains_key("Inter"), "labels lead with the measured face: {source}");
    }
}

#[test]
fn real_class_diagram_title_is_bold_through_the_stylesheet() {
    // Load-bearing: Merman sets the class title's weight in a class rule of
    // its `<style>`, not on the `<text>`.
    let svg = html_static_svg(CLASS);
    let used = collect(&svg);
    let inter = &used.families["Inter"];
    assert!(chars("Animal").is_subset(&inter.bold), "{inter:?}");
    assert!(chars("+String name").is_subset(&inter.regular), "{inter:?}");
}

#[test]
fn two_diagrams_sharing_and_differing_in_characters() {
    let first = collect(&html_static_svg("flowchart LR\n  A[Start] --> B[Stop]"));
    let second = collect(&html_static_svg("flowchart LR\n  A[Stop] --> B[Zebra]"));
    let first = only_inter(&first);
    let second = only_inter(&second);
    assert!(chars("Start Stop").is_subset(&first.regular), "{first:?}");
    assert!(!first.regular.contains(&'Z'));
    assert!(chars("Stop Zebra").is_subset(&second.regular), "{second:?}");
}

#[test]
fn a_page_without_text_draws_nothing() {
    assert_eq!(collect(&svg(r#"<rect width="10" height="10"/>"#)), UsedGlyphs::default());
    // Indentation between elements is not drawn text.
    assert_eq!(collect(&svg("<text>\n    <tspan>\n    </tspan>\n  </text>")), UsedGlyphs::default());
}

#[test]
fn text_and_tspans_are_collected_and_other_character_data_is_not() {
    let used = collect(&svg(
        "<title>Q</title><desc>R</desc><g><text>ab<tspan>cd</tspan></text></g><style>text{}</style>",
    ));
    let inter = only_inter(&used);
    assert_eq!(inter.regular, chars(" abcd"));
    assert!(inter.bold.is_empty());
}

#[test]
fn entities_and_character_references_are_decoded() {
    let used = collect(&svg("<text>a &amp; b &lt;&#x1F600;&#233;</text>"));
    assert_eq!(only_inter(&used).regular, chars("a&b<\u{1F600}\u{e9} "));
}

#[test]
fn controls_are_dropped_and_a_space_is_added() {
    let used = collect(&svg("<text>x\ty\u{85}z</text>"));
    assert_eq!(only_inter(&used).regular, chars("xyz "));
}

#[test]
fn bold_comes_from_attributes_inline_styles_and_rules() {
    let used = collect(&svg(concat!(
        r#"<style>.title { font-weight: bold } #id text { font-weight: 600 }</style>"#,
        r#"<text font-weight="bold">a</text>"#,
        r#"<text style="font-weight:700">b</text>"#,
        r#"<text class="title">c</text>"#,
        r#"<g id="id"><text>d</text></g>"#,
        r#"<g font-weight="bold"><text>e</text></g>"#,
        r#"<text font-weight="bolder">f</text>"#,
        r#"<text font-weight="599">g</text>"#,
        r#"<text style="font: bold 12px Inter">h</text>"#,
        r#"<g font-weight="bold"><text font-weight="lighter">i</text></g>"#,
    )));
    let inter = only_inter(&used);
    assert_eq!(inter.bold, chars("abcdefh "));
    assert_eq!(inter.regular, chars("gi "));
}

#[test]
fn the_cascade_follows_usvg_precedence() {
    let used = collect(&svg(concat!(
        // A rule beats the attribute; the style attribute beats a rule.
        r#"<style>.r { font-weight: bold } .n { font-weight: normal } .i { font-weight: bold !important }"#,
        r#" text.s { font-weight: normal } .s { font-weight: bold }</style>"#,
        r#"<text class="r" font-weight="normal">a</text>"#,
        r#"<text class="r" style="font-weight:normal">b</text>"#,
        // An important rule beats a later style attribute.
        r#"<text class="i" style="font-weight:normal">c</text>"#,
        // The more specific rule wins whatever its order.
        r#"<text class="s">d</text>"#,
        // An undefined value (a custom property) is ignored: inherited.
        r#"<g font-weight="bold"><text style="font-weight:var(--w)">e</text></g>"#,
        r#"<g font-weight="bold"><text font-weight="inherit">f</text></g>"#,
    )));
    let inter = only_inter(&used);
    assert_eq!(inter.bold, chars("ace f"));
    assert_eq!(inter.regular, chars("bd "));
}

#[test]
fn text_is_keyed_by_its_leading_family() {
    let used = collect(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"><style>.a {{ font-family: 'Arial', sans-serif }}</style>
        <text>x</text>
        <text class="a">y</text>
        <text font-family="{}">z</text>
        <g style="font-family: Helvetica Neue , serif"><text font-family="inherit">w</text></g></svg>"#,
        css_quoted("Odd\"Name")
    ));
    let families: Vec<&str> = used.families.keys().map(String::as_str).collect();
    assert_eq!(families, ["Arial", "Helvetica Neue", "Odd\"Name"], "text with no font-family is not recorded");
    assert_eq!(used.families["Arial"].regular, chars("y "));
    assert_eq!(used.for_family("arial").count(), 1, "family names match case-insensitively");
}

#[test]
fn first_family_reads_what_css_quoted_writes() {
    for name in ["Inter", "Odd\"Name", "back\\slash", "<b>&", "line\nbreak", "it's", "  spaced  "] {
        let list = format!("{}, sans-serif", css_quoted(name));
        assert_eq!(first_family(&list).as_deref(), Some(name), "{list}");
    }
    assert_eq!(first_family("\"trebuchet ms\", verdana").as_deref(), Some("trebuchet ms"));
    assert_eq!(first_family("  Helvetica   Neue , arial").as_deref(), Some("Helvetica Neue"));
    assert_eq!(first_family("sans-serif").as_deref(), Some("sans-serif"));
    assert_eq!(first_family(""), None);
    assert_eq!(first_family("  , arial"), None);
}

#[test]
fn malformed_svg_is_an_error() {
    assert!(collect_used_glyphs("<svg><text>unclosed</svg>").is_err());
    assert!(collect_used_glyphs("not xml").is_err());
}
