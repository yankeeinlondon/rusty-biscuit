//! The page's embedded font: one `@font-face` per family and weight the page
//! draws, covering the union of its diagrams' code points (AC-E3, R-E3), the
//! budget log (AC-E6), the failure path (AC-E7), the family Inter is declared
//! under (AC-E8), the page comments (AC-E14), and Inter's OFL line (AC-E15).
//!
//! Every embedded file is decoded from the CSS and read back with
//! `ttf-parser`, independently of the builder and of `allsorts`.

use std::collections::{BTreeMap, BTreeSet};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use ttf_parser::Face;

use super::embed_faces_tests::{arial, chain};
use super::font_chain_tests::MERMAID_STACK;
use super::postprocess_tests;
use crate::fonts::FontChain;
use crate::fonts::LabelMeasurer;
use crate::fonts::bundled::INTER_REGULAR;
use crate::mermaid::embed::embedding::{
    BUDGET_BYTES, FontEmbedding, FontEmbeddingError, build_font_embedding, build_font_embedding_with,
};
use crate::mermaid::embed::faces::{EmbeddedFaces, choose_embedded_faces};
use crate::mermaid::embed::glyphs::{UsedGlyphs, WeightGlyphs, collect_used_glyphs};
use crate::mermaid::embed::subset::SubsetError;
use crate::mermaid::postprocess::{html_static, postprocess};
use crate::tests::{capture_logs, capture_warnings};

const RULE_START: &str = "@font-face{font-family:";
const SRC_START: &str = "src:url(data:font/ttf;base64,";

/// A diagram drawing `regular` and `bold` in `family`.
fn drawn(family: &str, regular: &str, bold: &str) -> UsedGlyphs {
    UsedGlyphs {
        families: BTreeMap::from([(
            family.to_string(),
            WeightGlyphs {
                regular: regular.chars().collect(),
                bold: bold.chars().collect(),
            },
        )]),
    }
}

/// A chain with no installed face: every family is the bundled Inter.
fn inter_chain() -> FontChain {
    FontChain::injected(&[], &[], None)
}

fn faces(chain: &FontChain) -> EmbeddedFaces {
    choose_embedded_faces(&[MERMAID_STACK], &[], chain)
}

fn build(faces: &EmbeddedFaces, used: &[UsedGlyphs], chain: &FontChain) -> FontEmbedding {
    build_font_embedding(faces, used, chain).expect("the font embeds")
}

/// Each `@font-face` rule's `(family declaration, weight, decoded file)`.
fn rules(css: &str) -> Vec<(String, u16, Vec<u8>)> {
    css.lines()
        .filter(|line| line.starts_with("@font-face"))
        .map(|rule| {
            assert!(rule.starts_with(RULE_START), "{rule}");
            let family = rule[RULE_START.len()..].split(';').next().expect("family").to_string();
            let weight = rule
                .split("font-weight:")
                .nth(1)
                .and_then(|rest| rest.split(';').next())
                .expect("weight")
                .parse()
                .expect("numeric weight");
            let data = rule.split(SRC_START).nth(1).expect("a data URI");
            let (data, tail) = data.split_once(')').expect("url closes");
            assert_eq!(tail, " format(\"truetype\")}", "{rule}");
            (family, weight, BASE64.decode(data).expect("base64"))
        })
        .collect()
}

fn maps_all(file: &[u8], text: &str) -> bool {
    let face = Face::parse(file, 0).expect("the embedded file parses");
    text.chars().all(|c| face.glyph_index(c).is_some())
}

#[test]
fn diagrams_sharing_a_face_get_one_rule_per_weight_covering_their_union() {
    let chain = inter_chain();
    let faces = faces(&chain);
    let used = [drawn("Inter", "Ab ", "X "), drawn("Inter", "bZ ", ""), drawn("Inter", "", "Y ")];
    let embedding = build(&faces, &used, &chain);
    let rules = rules(&embedding.css);
    assert_eq!(embedding.css.matches("@font-face").count(), 2, "{}", embedding.css);
    assert_eq!(rules.len(), 2);
    let (family, weight, regular) = &rules[0];
    assert_eq!((family.as_str(), *weight), ("'Inter'", 400));
    assert!(maps_all(regular, "AbZ "));
    assert!(!maps_all(regular, "X") && !maps_all(regular, "Y"), "bold-only characters stay out");
    let (family, weight, bold) = &rules[1];
    assert_eq!((family.as_str(), *weight), ("'Inter'", 700));
    assert!(maps_all(bold, "XY "));
    assert!(!maps_all(bold, "A"));
    assert_eq!(embedding.rules.iter().map(|rule| rule.bytes).collect::<Vec<_>>(), [regular.len(), bold.len()]);
}

#[test]
fn a_bold_only_page_embeds_no_regular_rule() {
    let chain = inter_chain();
    let embedding = build(&faces(&chain), &[drawn("Inter", "", "Title ")], &chain);
    let rules = rules(&embedding.css);
    assert_eq!(rules.iter().map(|(_, weight, _)| *weight).collect::<Vec<_>>(), [700]);
}

#[test]
fn a_page_drawing_no_text_embeds_nothing() {
    let chain = inter_chain();
    for used in [vec![], vec![UsedGlyphs::default()], vec![drawn("Arial", "abc", "")]] {
        let embedding = build(&faces(&chain), &used, &chain);
        assert_eq!(embedding, FontEmbedding::default(), "{used:?}");
        assert_eq!(embedding.markup(), "");
    }
}

#[test]
fn diagrams_on_one_page_are_covered_by_the_subset() {
    // Two real diagrams through Merman and the HTML static post-process, over
    // the bundled-faces chain where `sans-serif` is Inter.
    let chain = postprocess_tests::chain();
    let sources = ["flowchart LR\n  A[Start café] --> B[Stop]", "classDiagram\n  class Zebra {\n    +String naïve\n  }"];
    let svgs: Vec<String> = sources
        .iter()
        .map(|source| {
            let raster = postprocess(&postprocess_tests::render(source, "default")).into_owned();
            html_static(&raster, &LabelMeasurer::new(chain)).into_owned()
        })
        .collect();
    let used: Vec<UsedGlyphs> = svgs.iter().map(|svg| collect_used_glyphs(svg).expect("parses")).collect();
    let faces = choose_embedded_faces(&[MERMAID_STACK], &[], chain);
    assert_eq!(faces.families[0].family, "Inter");
    let embedding = build(&faces, &used, chain);
    let rules = rules(&embedding.css);
    assert_eq!(rules.iter().map(|(_, weight, _)| *weight).collect::<Vec<_>>(), [400, 700], "the class title is bold");
    let inter = Face::parse(INTER_REGULAR, 0).expect("Inter parses");
    for (_, weight, file) in &rules {
        let subset = Face::parse(file, 0).expect("the subset parses");
        let page: BTreeSet<char> = used
            .iter()
            .flat_map(|diagram| diagram.for_family("Inter"))
            .flat_map(|glyphs| if *weight == 700 { &glyphs.bold } else { &glyphs.regular })
            .copied()
            .collect();
        assert!(!page.is_empty());
        for c in page.iter().filter(|&&c| inter.glyph_index(c).is_some()) {
            assert!(subset.glyph_index(*c).is_some(), "{c:?} at {weight}");
        }
    }
    let regular_page: String = used.iter().flat_map(|d| d.for_family("Inter")).flat_map(|g| &g.regular).collect();
    for label in ["Start", "café", "Stop", "naïve"] {
        assert!(label.chars().all(|c| regular_page.contains(c)), "{label} is drawn");
    }
}

#[test]
fn inter_is_declared_as_inter_with_its_ofl_line_immediately_before() {
    let chain = inter_chain();
    let embedding = build(&faces(&chain), &[drawn("Inter", "a", "b")], &chain);
    let lines: Vec<&str> = embedding.css.lines().collect();
    assert_eq!(lines.len(), 3, "{}", embedding.css);
    assert_eq!(
        lines[0],
        "/* Inter: Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter); SIL Open Font License 1.1 */"
    );
    assert!(lines[1].starts_with("@font-face{font-family:'Inter';font-style:normal;font-weight:400;"));
    assert!(lines[2].starts_with("@font-face{font-family:'Inter';font-style:normal;font-weight:700;"));
    assert_eq!(embedding.css.matches("Open Font License").count(), 1, "one line per page");
    assert_eq!(
        embedding.comments,
        ["<!-- mermaid font: \"Inter\" embedded; the font chain reached no installed face -->"]
    );
    assert_eq!(embedding.markup(), format!("{}\n<style>\n{}</style>\n", embedding.comments[0], embedding.css));
}

#[test]
fn a_page_embedding_only_a_host_face_has_no_ofl_line() {
    let installed = arial(0x0008, 0x0008);
    let chain = chain(&installed, None);
    let embedding = build(&faces(&chain), &[drawn("Arial", "Hi", "")], &chain);
    assert!(!embedding.css.contains("/*"), "{}", embedding.css);
    assert!(embedding.css.starts_with("@font-face{font-family:'Arial';"));
    assert_eq!(
        embedding.comments,
        ["<!-- mermaid font: \"Arial\" embedded; the face the font chain resolved, fsType 0x0008 (Editable) -->"]
    );
}

#[test]
fn a_restricted_resolved_face_embeds_inter_and_says_why() {
    let installed = arial(0x0002, 0x0002);
    let chain = chain(&installed, None);
    // Forced, as a render drawing the refused face forces it.
    let faces = choose_embedded_faces(&[MERMAID_STACK], &[MERMAID_STACK.to_string()], &chain);
    let embedding = build(&faces, &[drawn("Inter", "Animal", "")], &chain);
    assert_eq!(
        embedding.comments,
        ["<!-- mermaid font: \"Inter\" embedded; resolved face \"Arial\" is Restricted (fsType 0x0002) -->"]
    );
    assert!(embedding.css.starts_with("/* Inter: "));
    assert!(!embedding.css.contains("'Arial'"));
}

#[test]
fn a_family_the_page_never_draws_is_not_embedded() {
    let installed = arial(0x0000, 0x0000);
    let chain = chain(&installed, None);
    let faces = choose_embedded_faces(&[MERMAID_STACK, "Inter, sans-serif"], &[], &chain);
    assert_eq!(faces.families.len(), 2);
    let embedding = build(&faces, &[drawn("Arial", "x", "")], &chain);
    assert_eq!(rules(&embedding.css).len(), 1);
    assert!(!embedding.css.contains("'Inter'"), "{}", embedding.css);
    assert_eq!(embedding.comments.len(), 1);
}

#[test]
fn a_no_subsetting_face_is_embedded_whole_logged_and_commented() {
    let installed = arial(0x0100, 0x0100);
    let chain = chain(&installed, None);
    let (embedding, infos) = capture_logs(tracing::Level::INFO, || {
        build(&faces(&chain), &[drawn("Arial", "a", "")], &chain)
    });
    let rules = rules(&embedding.css);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].2, installed[0], "the whole face, byte for byte");
    assert!(embedding.rules[0].unsubset);
    assert!(
        embedding.comments[0].ends_with("; embedded unsubset because fsType 0x0100 forbids subsetting -->"),
        "{:?}",
        embedding.comments
    );
    assert!(infos.iter().any(|message| message.contains("unsubset")), "{infos:?}");
    // The whole face is over budget too, and still embedded.
    assert!(infos.iter().any(|message| message.contains("over the 60000-byte budget")), "{infos:?}");
}

#[test]
fn a_subset_over_budget_is_embedded_and_its_size_logged() {
    let inter = Face::parse(INTER_REGULAR, 0).expect("Inter parses");
    let mut everything = String::new();
    inter.tables().cmap.expect("cmap").subtables.into_iter().filter(|table| table.is_unicode()).for_each(|table| {
        table.codepoints(|code_point| everything.extend(char::from_u32(code_point)));
    });
    let chain = inter_chain();
    let (embedding, infos) =
        capture_logs(tracing::Level::INFO, || build(&faces(&chain), &[drawn("Inter", &everything, "")], &chain));
    let bytes = embedding.rules[0].bytes;
    assert!(bytes > BUDGET_BYTES, "{bytes}");
    assert_eq!(rules(&embedding.css)[0].2.len(), bytes);
    let expected = format!("the embedded font \"Inter\" weight 400 is {bytes} bytes, over the 60000-byte budget");
    assert!(infos.iter().any(|message| message.contains(&expected)), "{infos:?}");
}

#[test]
fn a_subset_within_budget_logs_nothing() {
    let chain = inter_chain();
    let (_, infos) = capture_logs(tracing::Level::INFO, || build(&faces(&chain), &[drawn("Inter", "abc", "")], &chain));
    assert!(infos.is_empty(), "{infos:?}");
}

#[test]
fn a_subsetting_failure_embeds_nothing_and_warns() {
    let chain = inter_chain();
    let failing = |_: &[u8], _: u32, _: Option<&BTreeSet<char>>| Err(SubsetError::Subset("injected".to_string()));
    let (result, warnings) = capture_warnings(|| {
        build_font_embedding_with(&faces(&chain), &[drawn("Inter", "a", "b")], &chain, failing)
    });
    assert_eq!(
        result,
        Err(FontEmbeddingError::Subset {
            family: "Inter".to_string(),
            weight: 400,
            source: SubsetError::Subset("injected".to_string()),
        })
    );
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("cannot be embedded"), "{warnings:?}");
    // A failure on the second weight also fails the whole page.
    let second_fails = |data: &[u8], index: u32, code_points: Option<&BTreeSet<char>>| match code_points {
        Some(code_points) if code_points.contains(&'b') => Err(SubsetError::Subset("bold".to_string())),
        Some(code_points) => crate::mermaid::embed::subset::subset(data, index, code_points),
        None => unreachable!("Inter is subset"),
    };
    let (result, _) = capture_warnings(|| {
        build_font_embedding_with(&faces(&chain), &[drawn("Inter", "a", "b")], &chain, second_fails)
    });
    assert!(matches!(result, Err(FontEmbeddingError::Subset { weight: 700, .. })), "{result:?}");
}

#[test]
fn the_css_references_nothing_outside_the_page() {
    let installed = arial(0x0000, 0x0000);
    let chain = chain(&installed, None);
    let faces = choose_embedded_faces(&[MERMAID_STACK, "Inter"], &[], &chain);
    let embedding = build(&faces, &[drawn("Arial", "ab", "cd"), drawn("Inter", "ef", "gh")], &chain);
    assert_eq!(rules(&embedding.css).len(), 4);
    let urls = embedding.css.matches("url(").count();
    assert_eq!(urls, 4);
    assert_eq!(embedding.css.matches("url(data:font/ttf;base64,").count(), urls);
    // The OFL comment names Inter's project page; no rule references it.
    let rules_only: String = embedding.css.lines().filter(|line| !line.starts_with("/*")).collect();
    for external in ["http:", "https:", "url(//", "@import"] {
        assert!(!rules_only.contains(external), "{external}");
    }
    assert!(!embedding.markup().contains("</style>\n<style>"), "one style element");
}

/// A chain whose only installed face is a permitted regular `Arial`, which
/// the chain resolves for both weights.
fn regular_only_arial_chain() -> FontChain {
    let [regular, _] = arial(0x0000, 0x0000);
    chain(&[regular], None)
}

/// The `(family declaration, weight)` of each rule, in order.
fn rule_keys(css: &str) -> Vec<(String, u16)> {
    rules(css).into_iter().map(|(family, weight, _)| (family, weight)).collect()
}

#[test]
fn one_face_serving_both_weights_gets_a_rule_per_weight_drawn() {
    let chain = regular_only_arial_chain();
    let faces = faces(&chain);
    let choice = &faces.families[0];
    assert_eq!(choice.family, "Arial");
    assert_eq!(choice.regular.id, choice.bold.id, "precondition: one face for both weights");
    let embedding = build(&faces, &[drawn("Arial", "+String name ", "Zebra ")], &chain);
    let rules = rules(&embedding.css);
    assert_eq!(rule_keys(&embedding.css), [("'Arial'".to_string(), 400), ("'Arial'".to_string(), 700)]);
    assert!(maps_all(&rules[0].2, "+String name "));
    assert!(!maps_all(&rules[0].2, "Z"), "the regular subset keeps regular characters only");
    assert!(maps_all(&rules[1].2, "Zebra "), "the bold rule carries the bold-only characters");
    assert_eq!(embedding.comments.len(), 1, "one comment per family");
}

#[test]
fn bold_only_use_of_a_face_serving_both_weights_embeds_one_bold_rule() {
    let chain = regular_only_arial_chain();
    let embedding = build(&faces(&chain), &[drawn("Arial", "", "Zebra ")], &chain);
    assert_eq!(rule_keys(&embedding.css), [("'Arial'".to_string(), 700)]);
    assert!(maps_all(&rules(&embedding.css)[0].2, "Zebra "));
}

#[test]
fn requested_families_choosing_one_family_share_its_rules_and_union() {
    for chain in [regular_only_arial_chain(), chain(&arial(0x0000, 0x0000), None)] {
        let faces = choose_embedded_faces(&[MERMAID_STACK, "Arial, sans-serif"], &[], &chain);
        assert_eq!(faces.families.len(), 2);
        assert!(faces.families.iter().all(|choice| choice.family == "Arial"));
        let used = [drawn("Arial", "ab ", "Z "), drawn("Arial", "cd ", "Y ")];
        let embedding = build(&faces, &used, &chain);
        let rules = rules(&embedding.css);
        assert_eq!(rule_keys(&embedding.css), [("'Arial'".to_string(), 400), ("'Arial'".to_string(), 700)]);
        assert!(maps_all(&rules[0].2, "abcd "));
        assert!(maps_all(&rules[1].2, "YZ "));
        assert_eq!(embedding.comments.len(), 1, "{:?}", embedding.comments);
    }
}

/// A refused face is never embedded, even when glyphs reach it (which a
/// render prevents by forcing the family to Inter): debug builds assert, and
/// release builds skip the weight.
#[test]
#[cfg_attr(debug_assertions, should_panic(expected = "a refused face"))]
fn a_refused_face_is_never_embedded() {
    let chain = chain(&arial(0x0000, 0x0002), None);
    let faces = faces(&chain);
    let (embedding, _) = capture_warnings(|| build(&faces, &[drawn("Arial", "ab", "cd")], &chain));
    assert_eq!(rule_keys(&embedding.css), [("'Arial'".to_string(), 400)]);
}

/// An installed face named Inter is not chosen, so a page whose other family
/// falls back to the bundled Inter declares one face per weight as `Inter`.
#[test]
fn an_installed_inter_and_the_bundled_inter_never_share_a_page() {
    let mut installed = arial(0x0002, 0x0002).to_vec();
    installed.extend(super::embed_faces_tests::installed_inter());
    let chain = chain(&installed, None);
    let faces = choose_embedded_faces(&[MERMAID_STACK, "Inter, sans-serif"], &[MERMAID_STACK.to_string()], &chain);
    assert!(faces.families.iter().all(|choice| choice.family == "Inter" && choice.bundled_inter), "{faces:?}");
    let embedding = build(&faces, &[drawn("Inter", "ab", "cd")], &chain);
    assert_eq!(rule_keys(&embedding.css), [("'Inter'".to_string(), 400), ("'Inter'".to_string(), 700)]);
    assert!(embedding.rules.iter().all(|rule| chain.is_bundled_inter(rule.id)), "{:?}", embedding.rules);
}
