//! How the Markdown and HTML folds write a link's query metadata.
//!
//! The Markdown fold writes it as a JSON5 structured title, which an
//! independent CommonMark reader (`pulldown-cmark`) must hand back as exactly
//! that JSON5 text, and `parse_link_title` must read back to the same query.
//! The HTML fold writes it into the `href` query string, moving the fragment
//! to `heading=`. A link without metadata is written as before.

use pulldown_cmark::{Event, Options, Parser, Tag};
use renderable::output::link_metadata::{
    LinkMetadata, QueryParams, QueryValue, StructuredTitle, parse_link_title,
};
use renderable::tree::{
    BrowserRenderOptions, ColumnAlign, Document, MarkdownDialect, MarkdownRenderOptions,
    RenderNode, render_browser_document_html, render_browser_node, render_markdown_node,
};

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

fn query(pairs: impl IntoIterator<Item = (&'static str, QueryValue)>) -> QueryParams {
    pairs.into_iter().collect()
}

fn located(url: &str, title: Option<&str>, query: QueryParams) -> RenderNode {
    let mut link = RenderNode::link(
        url,
        title.map(str::to_owned),
        vec![RenderNode::text("types.md:14:83")],
    );
    link.attrs.set_link_metadata(LinkMetadata { query });
    link
}

fn line_and_column() -> QueryParams {
    query([("line", 14_u64.into()), ("column", 83_u64.into())])
}

fn markdown(node: &RenderNode, dialect: MarkdownDialect) -> String {
    let opts = MarkdownRenderOptions {
        dialect,
        ..MarkdownRenderOptions::default()
    };
    render_markdown_node(node, &opts)
        .expect("the tree renders")
        .output
}

/// The title of every link a reader finds in `markdown`.
fn read_titles(markdown: &str) -> Vec<String> {
    Parser::new_ext(markdown, Options::ENABLE_TABLES)
        .filter_map(|event| match event {
            Event::Start(Tag::Link { title, .. }) => Some(title.to_string()),
            _ => None,
        })
        .collect()
}

fn in_paragraph(node: RenderNode) -> RenderNode {
    RenderNode::root(vec![RenderNode::paragraph(vec![node])])
}

fn in_cell(node: RenderNode) -> RenderNode {
    RenderNode::root(vec![RenderNode::table(
        vec![ColumnAlign::None],
        vec![
            RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("h")])]),
            RenderNode::table_row(vec![RenderNode::table_cell(vec![node])]),
        ],
    )])
}

/// Every `href` value in `html`, with `&amp;` decoded.
fn hrefs(html: &str) -> Vec<String> {
    html.split("href=\"")
        .skip(1)
        .map(|rest| rest[..rest.find('"').expect("a closed href")].replace("&amp;", "&"))
        .collect()
}

/// The `href`s of `node` through both browser paths, which must agree.
fn html_hrefs(node: &RenderNode) -> Vec<String> {
    let opts = BrowserRenderOptions::default();
    let fragment = render_browser_node(node, &opts)
        .expect("the tree renders")
        .output
        .render();
    let document = render_browser_document_html(&Document::new(node.clone()), &opts)
        .expect("the document renders")
        .output;
    assert_eq!(hrefs(&fragment), hrefs(&document));
    assert!(!fragment.contains("data-"), "{fragment}");
    hrefs(&fragment)
}

#[test]
fn a_located_link_writes_a_single_quoted_json5_title() {
    let tree = in_paragraph(located("/abs/types.md#slug", None, line_and_column()));
    for dialect in DIALECTS {
        assert_eq!(
            markdown(&tree, dialect),
            "[types.md:14:83](/abs/types.md#slug '{ qp: { line: 14, column: 83 } }')"
        );
    }
}

#[test]
fn the_json5_title_reads_back_to_the_same_query_and_tooltip() {
    let cases = [
        (None, line_and_column()),
        (Some("Types"), line_and_column()),
        (
            Some("it's a \\ path"),
            query([("q", QueryValue::from("o'k \\x \"y\" 'z'"))]),
        ),
        (
            Some("ends in \\"),
            query([("flag", true.into()), ("a&b", "&copy;".into())]),
        ),
        (Some("pipe | here"), query([("p", "a|b".into())])),
    ];
    for (title, query) in cases {
        let expected = StructuredTitle {
            title: title.map(str::to_owned),
            query: query.clone(),
            ..StructuredTitle::default()
        };
        for tree in [
            in_paragraph(located("/a.md#h", title, query.clone())),
            in_cell(located("/a.md#h", title, query.clone())),
        ] {
            for dialect in DIALECTS {
                let out = markdown(&tree, dialect);
                let titles = read_titles(&out);
                assert_eq!(titles, vec![expected.to_json5()], "{out}");
                let parsed = parse_link_title(&titles[0]);
                assert_eq!(parsed.notice, None, "{out}");
                let structured = parsed.structured().expect("a structured title");
                assert_eq!(structured.query, query, "{out}");
                assert_eq!(parsed.tooltip(), title, "{out}");
            }
        }
    }
}

#[test]
fn a_link_without_metadata_keeps_its_plain_title() {
    let tree = in_paragraph(RenderNode::link(
        "/a.md#h",
        Some("Plain".into()),
        vec![RenderNode::text("a")],
    ));
    for dialect in DIALECTS {
        assert_eq!(markdown(&tree, dialect), r#"[a](/a.md#h "Plain")"#);
    }
    assert_eq!(html_hrefs(&tree), vec!["/a.md#h"]);
}

#[test]
fn html_moves_the_fragment_and_query_into_the_href() {
    let tree = in_paragraph(located(
        "/x/types.md#simplified-schema-types",
        None,
        line_and_column(),
    ));
    assert_eq!(
        html_hrefs(&tree),
        vec!["/x/types.md?heading=simplified-schema-types&line=14&column=83"]
    );
}

#[test]
fn html_keeps_unrelated_query_pairs_first_and_lets_metadata_win_collisions() {
    let tree = in_paragraph(located(
        "/x/a.md?b=1&line=9&a=two%20words#frag",
        None,
        query([
            ("zeta", "z".into()),
            ("column", 3_u64.into()),
            ("line", 14_u64.into()),
            ("alpha", true.into()),
        ]),
    ));
    assert_eq!(
        html_hrefs(&tree),
        vec!["/x/a.md?b=1&a=two%20words&heading=frag&line=14&column=3&alpha=true&zeta=z"]
    );
}

#[test]
fn html_writes_one_heading_and_an_explicit_one_wins() {
    let explicit = in_paragraph(located(
        "/a.md?heading=old#from-fragment",
        None,
        query([("heading", "explicit".into())]),
    ));
    assert_eq!(html_hrefs(&explicit), vec!["/a.md?heading=explicit"]);

    let from_fragment = in_paragraph(located(
        "/a.md?heading=old#caf%C3%A9",
        None,
        query([("line", 1_u64.into())]),
    ));
    assert_eq!(
        html_hrefs(&from_fragment),
        vec!["/a.md?heading=caf%C3%A9&line=1"]
    );
}

#[test]
fn html_percent_encodes_query_values() {
    let tree = in_paragraph(located(
        "https://example.com/docs.md",
        None,
        query([("q", "a b&c=d".into())]),
    ));
    assert_eq!(
        html_hrefs(&tree),
        vec!["https://example.com/docs.md?q=a+b%26c%3Dd"]
    );
}

#[test]
fn an_apostrophe_in_the_title_is_escaped_for_the_single_quotes() {
    let mut link = RenderNode::link(
        "/abs/types.md",
        Some("it's here".into()),
        vec![RenderNode::text("Types")],
    );
    link.attrs.set_link_metadata(LinkMetadata {
        query: query([("line", 3_u64.into())]),
    });
    assert_eq!(
        markdown(&in_paragraph(link), MarkdownDialect::Markdown),
        r#"[Types](/abs/types.md '{ title: "it\'s here", qp: { line: 3 } }')"#
    );
}
