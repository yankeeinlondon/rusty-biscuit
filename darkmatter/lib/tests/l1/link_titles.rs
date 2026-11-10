//! Structured link titles through public results: what a title means, how
//! `qp` lowers to each target, and that `Link`'s convenience methods and the
//! render tree agree.

use biscuit_terminal::render_tree::render_terminal_output;
use darkmatter::markdown::{Markdown, output};
use darkmatter::render::{Link, LinkTarget};
use renderable::color::{ColorDepth, ColorMode};
use renderable::output::link_metadata::{LinkMetadata, LinkTitleNotice, QueryParams, QueryValue};
use renderable::output::{OutputOptions, TerminalProfile, Width};
use renderable::tree::{NodeKind, RenderNode, RenderStrictness};

fn hyperlink_profile() -> TerminalProfile {
    TerminalProfile {
        color_depth: ColorDepth::TrueColor,
        color_mode: ColorMode::Dark,
        text_color: None,
        background_color: None,
        is_tty: true,
        hyperlinks: true,
        italics: true,
        underline: true,
        graphics: false,
        unicode: true,
    }
}

fn find_link(node: &RenderNode) -> Option<&RenderNode> {
    if matches!(node.kind, NodeKind::Link { .. }) {
        return Some(node);
    }
    node.children().iter().find_map(find_link)
}

fn html_of(text: &str) -> String {
    let md: Markdown = text.into();
    md.as_html(output::HtmlOptions::default()).expect("renders")
}

fn terminal_of(text: &str) -> String {
    let md: Markdown = text.into();
    let document = md.as_document().expect("builds");
    render_terminal_output(
        &document,
        &OutputOptions::new(Width::Columns(120), hyperlink_profile()),
        RenderStrictness::Warn,
    )
    .expect("renders")
    .output
}

fn query(pairs: &[(&str, &str)]) -> LinkMetadata {
    LinkMetadata { query: pairs.iter().copied().collect::<QueryParams>() }
}

#[test]
fn spec_plain_and_structured_examples() {
    let plain = Link::try_from(r#"[Plain title](./a.md "Shown as a tooltip")"#).unwrap();
    assert_eq!(plain.title(), Some("Shown as a tooltip"));
    assert_eq!(plain.title_notice(), None);

    let structured =
        Link::try_from(r#"[Structured](./a.md '{ class: "docs-link", target: "_blank", qp: { line: 14 } }')"#).unwrap();
    assert_eq!(structured.class(), Some("docs-link"));
    assert_eq!(structured.target(), Some(&LinkTarget::Blank));
    assert_eq!(structured.title(), None);
    assert_eq!(structured.link_metadata().query.get("line"), Some(&QueryValue::from(14_u64)));
    assert_eq!(structured.title_notice(), None);
}

#[test]
fn spec_legacy_example_is_a_plain_title_with_no_attributes() {
    let raw = "class='docs-link' target='_blank' data-section='help'";
    let link = Link::try_from(format!(r#"[Docs](https://example.com/docs "{raw}")"#)).unwrap();
    assert_eq!(link.title(), Some(raw));
    assert_eq!(link.class(), None);
    assert_eq!(link.target(), None);
    assert!(link.data().is_empty());
    assert_eq!(link.title_notice(), Some(&LinkTitleNotice::Legacy));

    let html = html_of(&format!("[Docs](https://example.com/docs \"{raw}\")\n"));
    assert!(html.contains("title="), "the text stays the tooltip: {html}");
    assert!(!html.contains(r#"class="docs-link""#), "{html}");
    assert!(!html.contains(r#"target="_blank""#), "{html}");
}

#[test]
fn a_brace_that_does_not_parse_is_plain_with_a_malformed_notice() {
    let link = Link::with_title_parsed("x", "./a.md", "{ class: ").unwrap();
    assert_eq!(link.title(), Some("{ class:"));
    assert_eq!(link.class(), None);
    assert!(matches!(link.title_notice(), Some(LinkTitleNotice::Malformed { .. })));
}

#[test]
fn a_non_object_json5_title_is_plain_without_a_notice() {
    for raw in ["42", "[1, 2]", "true", "'quoted'"] {
        let link = Link::with_title_parsed("x", "./a.md", raw).unwrap();
        assert_eq!(link.title(), Some(raw));
        assert_eq!(link.title_notice(), None, "{raw}");
    }
}

#[test]
fn unknown_keys_are_ignored_without_a_notice() {
    let link = Link::with_title_parsed("x", "./a.md", "{ class: 'a', colour: 'red', nested: { y: 1 } }").unwrap();
    assert_eq!(link.class(), Some("a"));
    assert_eq!(link.title_notice(), None);
    assert!(link.data().is_empty());
}

#[test]
fn an_unusable_qp_is_ignored_with_a_malformed_notice() {
    for raw in ["{ qp: 3 }", "{ qp: [1] }", "{ qp: { a: null } }", "{ qp: { a: [1] } }", "{ qp: { a: { b: 1 } } }"] {
        let link = Link::with_title_parsed("x", "./a.md", raw).unwrap();
        assert!(link.link_metadata().is_empty(), "{raw}");
        assert!(matches!(link.title_notice(), Some(LinkTitleNotice::Malformed { .. })), "{raw}");
    }
}

#[test]
fn html_query_order_is_heading_then_line_column_then_sorted() {
    let text = "[x](./types.md?keep=1&line=9#setup '{ qp: { zeta: \"z\", column: 83, alpha: true, line: 14 } }')\n";
    let expected = "./types.md?keep=1&amp;heading=setup&amp;line=14&amp;column=83&amp;alpha=true&amp;zeta=z";

    let link = Link::try_from(text.trim_end()).unwrap();
    assert!(link.to_html().contains(&format!(r#"href="{expected}""#)), "{}", link.to_html());

    let html = html_of(text);
    assert!(html.contains(&format!(r#"href="{expected}""#)), "{html}");
    assert!(!html.contains("#setup"), "no fragment: {html}");
    assert!(!html.contains("data-line"), "no transport data attribute: {html}");
}

#[test]
fn html_metadata_heading_replaces_the_fragment_once() {
    let link = Link::with_title_parsed("x", "./a.md?heading=old#frag", "{ qp: { heading: 'new' } }").unwrap();
    assert_eq!(link.to_html(), r#"<a href="./a.md?heading=new">x</a>"#);
}

#[test]
fn a_link_without_qp_keeps_its_fragment_everywhere() {
    let link = Link::with_title_parsed("x", "./a.md#setup", "{ class: 'c' }").unwrap();
    assert!(link.to_html().contains(r##"href="./a.md#setup""##));
    assert!(link.to_terminal_unchecked().contains("\x1b]8;;./a.md#setup\x07"));

    assert!(html_of("[x](./a.md#setup \"Tip\")\n").contains(r##"href="./a.md#setup""##));
    assert!(terminal_of("[x](./a.md#setup \"Tip\")\n").contains("./a.md#setup"));
}

#[test]
fn osc8_carries_safe_parameters_with_a_bare_uri() {
    let link = Link::new("doc", "https://example.com/doc.md?x=1#setup").unwrap().with_link_metadata(query(&[
        ("zeta", "z"),
        ("line", "14"),
        ("colon", "a:b"),
        ("semi", "a;b"),
        ("control", "a\u{7}b"),
    ]));
    assert_eq!(
        link.to_terminal_unchecked(),
        "\x1b]8;line=14:heading=setup:zeta=z;https://example.com/doc.md\x07doc\x1b]8;;\x07"
    );
}

#[test]
fn the_tree_path_lowers_a_qp_title_to_the_same_osc8_parameters() {
    let out = terminal_of("[doc](https://example.com/doc.md#setup '{ qp: { line: 14, bad: \"a:b\", zeta: \"z\" } }')\n");
    assert!(out.contains("\x1b]8;line=14:heading=setup:zeta=z;https://example.com/doc.md\x1b\\"), "{out:?}");
}

#[test]
fn every_tree_fold_receives_typed_metadata_and_no_raw_title() {
    let md: Markdown = "[x](./a.md '{ title: \"Tip\", qp: { line: 3 } }')\n".into();
    let document = md.as_document().unwrap();
    let link = find_link(&document.root).expect("a link");
    assert_eq!(link.attrs.link_metadata().unwrap().query.get("line"), Some(&QueryValue::from(3_u64)));
    let NodeKind::Link { title, .. } = &link.kind else { unreachable!() };
    assert_eq!(title.as_deref(), Some("Tip"));
}

#[test]
fn a_windows_file_uri_stays_forward_slashed_on_every_os() {
    let uri = "file:///C:/src/rusty-biscuit/simplified-schema/docs/topics/types.md";
    let link = Link::new("types.md:14:83", uri).unwrap().with_link_metadata(query(&[("line", "14"), ("column", "83")]));
    let out = link.to_terminal_unchecked();
    assert!(out.starts_with(&format!("\x1b]8;line=14:column=83;{uri}\x07")), "{out:?}");
    assert!(!out.contains('\\'), "{out:?}");
    assert!(link.to_html().contains(r#"href="file:///C:/src/rusty-biscuit/simplified-schema/docs/topics/types.md?line=14&amp;column=83""#));
}

#[test]
fn an_encoded_metadata_package_still_decodes_and_writes_json5() {
    // `{"__link_ref":"darkmatter:link:v1","class":"chip","target":"_blank","title":"Tip","data":{"id":"abc"}}`
    let encoded = "eyJfX2xpbmtfcmVmIjoiZGFya21hdHRlcjpsaW5rOnYxIiwiY2xhc3MiOiJjaGlwIiwidGFyZ2V0IjoiX2JsYW5rIiwidGl0bGUiOiJUaXAiLCJkYXRhIjp7ImlkIjoiYWJjIn19";
    let link = Link::try_from(format!(r#"[x](./a.md "{encoded}")"#)).unwrap();
    assert_eq!(link.class(), Some("chip"));
    assert_eq!(link.target(), Some(&LinkTarget::Blank));
    assert_eq!(link.title(), Some("Tip"));
    assert_eq!(link.data().get("id").map(String::as_str), Some("abc"));
    assert_eq!(link.title_notice(), None);

    let written = link.to_markdown(false);
    assert_eq!(written, r#"[x](./a.md '{ title: "Tip", class: "chip", target: "_blank", "data-id": "abc" }')"#);
    assert_eq!(Link::try_from(written.as_str()).unwrap(), link);
}

#[test]
fn a_padded_encoded_package_is_not_mistaken_for_a_legacy_title() {
    // `{"__link_ref":"darkmatter:link:v1","class":"a"}`: base64 padding is an `=`.
    let encoded = "eyJfX2xpbmtfcmVmIjoiZGFya21hdHRlcjpsaW5rOnYxIiwiY2xhc3MiOiJhIn0=";
    let link = Link::with_title_parsed("x", "./a.md", encoded).unwrap();
    assert_eq!(link.class(), Some("a"));
    assert_eq!(link.title_notice(), None);
    let md: Markdown = format!("[x](./a.md \"{encoded}\")\n").as_str().into();
    assert!(md.link_title_warnings().is_empty());
}

/// Parse, write, parse, write: the second write is the first, and the links
/// are equal, for each title form.
#[test]
fn parse_write_parse_is_stable() {
    for text in [
        r#"[x](./a.md)"#,
        r#"[x](./a.md "Plain tip")"#,
        r#"[x](./a.md '{ qp: { line: 14, column: 83 } }')"#,
        r#"[x](./a.md '{ "class": "c", "qp": { "ok": true, "n": 1.5 } }')"#,
        r#"[x](./a.md '{ title: "It\'s \"quoted\"", class: "c", target: "_blank", "data-id": "1 (one)", qp: { h: "a b)" } }')"#,
    ] {
        let first = Link::try_from(text).unwrap();
        let written = first.to_markdown(false);
        let second = Link::try_from(written.as_str()).unwrap();
        assert_eq!(first, second, "{text} -> {written}");
        assert_eq!(second.to_markdown(false), written, "{text}");
    }
}

#[test]
fn a_location_title_round_trips_exactly() {
    let text = "[types.md:14:83](./types.md '{ qp: { line: 14, column: 83 } }')";
    assert_eq!(Link::try_from(text).unwrap().to_markdown(false), text);
}
