//! Raw inline `Html` holding a line ending, inside an ATX heading or a GFM
//! table cell.
//!
//! Both contexts are one Markdown line: a line feed, a carriage return, or
//! a CRLF written there ends the heading or the table row, and a reader sees
//! different structure. A raw payload's bytes are the author's, so a line
//! ending in one cannot be kept faithfully: [`RenderStrictness::Strict`]
//! rejects it, and `Warn` and `Lossy` write each ending as a character
//! reference (`&#13;`, `&#10;`), `Warn` recording a lossy diagnostic with the
//! payload's span. A payload without a line ending stays byte for byte.
//!
//! Each case is checked on the rendered bytes and on an independent
//! CommonMark/GFM reading (`pulldown-cmark`) of the block structure.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::tree::{
    ColumnAlign, Diagnostic, HeadingDepth, MarkdownDialect, MarkdownRenderOptions, RenderError,
    RenderNode, RenderStrictness, SourceId, SourceLocation, SourceSpan, render_markdown_node,
};

/// The start of the diagnostic message a raw line ending in one of these
/// contexts produces.
const LINE_ENDING: &str = "raw HTML holds a line ending";

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

fn opts(dialect: MarkdownDialect, strictness: RenderStrictness) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        dialect,
        strictness,
        style: None,
    }
}

/// A span distinct per raw node, so a diagnostic can be traced to its node.
fn span_at(index: usize) -> SourceSpan {
    SourceSpan {
        location: Some(SourceLocation {
            source: SourceId(7),
            bytes: index * 100..index * 100 + 10,
        }),
        ..SourceSpan::synthetic()
    }
}

/// One inline raw node per fragment, each with its own span.
fn raws(fragments: &[&str]) -> Vec<RenderNode> {
    fragments
        .iter()
        .enumerate()
        .map(|(index, fragment)| {
            let mut node = RenderNode::html(*fragment, false);
            node.span = span_at(index);
            node
        })
        .collect()
}

/// Payload fragments by name: each list concatenates to one payload.
fn payloads() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("LF", vec!["<span title=\"a\nb\">t</span>"]),
        ("CR", vec!["<span title=\"a\rb\">t</span>"]),
        ("CRLF", vec!["<span title=\"a\r\nb\">t</span>"]),
        (
            "CRLF split at CR|LF",
            vec!["<span title=\"a\r", "\nb\">t</span>"],
        ),
        ("line ending between tags", vec!["<b>x</b>\n<i>y</i>"]),
    ]
}

/// The payload fragments as `Warn` and `Lossy` write them.
fn encoded(fragments: &[&str]) -> String {
    fragments
        .concat()
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
}

/// How many fragments hold a line ending, which is how many diagnostics
/// `Warn` records.
fn holding_endings(fragments: &[&str]) -> usize {
    fragments
        .iter()
        .filter(|fragment| fragment.contains(['\r', '\n']))
        .count()
}

/// The single-line structure a context sits in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Heading,
    Table,
}

type Place = fn(Vec<RenderNode>) -> RenderNode;

fn depth() -> HeadingDepth {
    HeadingDepth::new(2).expect("valid depth")
}

/// A two-row, two-column table with `cell` as the first body cell.
fn table_with_body_cell(cell: Vec<RenderNode>) -> RenderNode {
    RenderNode::table(
        vec![ColumnAlign::None, ColumnAlign::None],
        vec![
            RenderNode::table_row(vec![
                RenderNode::table_cell(vec![RenderNode::text("H1")]),
                RenderNode::table_cell(vec![RenderNode::text("H2")]),
            ]),
            RenderNode::table_row(vec![
                RenderNode::table_cell(cell),
                RenderNode::table_cell(vec![RenderNode::text("z")]),
            ]),
        ],
    )
}

/// Every single-line context and every way raw HTML reaches it. Each place
/// is followed by a paragraph, so a context that ends early shows up as
/// extra structure before it.
fn placements() -> Vec<(&'static str, Shape, Place)> {
    vec![
        ("heading", Shape::Heading, |raw| {
            RenderNode::heading(depth(), raw)
        }),
        ("section heading", Shape::Heading, |raw| {
            RenderNode::section(depth(), raw, vec![])
        }),
        ("heading > strong", Shape::Heading, |raw| {
            let mut children = vec![RenderNode::text("s")];
            children.extend(raw);
            RenderNode::heading(depth(), vec![RenderNode::strong(children)])
        }),
        ("heading > link", Shape::Heading, |raw| {
            RenderNode::heading(depth(), vec![RenderNode::link("https://x.test", None, raw)])
        }),
        ("heading > classed span", Shape::Heading, |raw| {
            RenderNode::heading(depth(), vec![RenderNode::span(vec!["note".into()], raw)])
        }),
        ("table header cell", Shape::Table, |raw| {
            RenderNode::table(
                vec![ColumnAlign::None, ColumnAlign::None],
                vec![
                    RenderNode::table_row(vec![
                        RenderNode::table_cell(raw),
                        RenderNode::table_cell(vec![RenderNode::text("H2")]),
                    ]),
                    RenderNode::table_row(vec![
                        RenderNode::table_cell(vec![RenderNode::text("y")]),
                        RenderNode::table_cell(vec![RenderNode::text("z")]),
                    ]),
                ],
            )
        }),
        ("table body cell", Shape::Table, table_with_body_cell),
        ("table cell > strong", Shape::Table, |raw| {
            let mut children = vec![RenderNode::text("s")];
            children.extend(raw);
            table_with_body_cell(vec![RenderNode::strong(children)])
        }),
        ("table cell > link", Shape::Table, |raw| {
            table_with_body_cell(vec![RenderNode::link("https://x.test", None, raw)])
        }),
        ("table cell > classed span", Shape::Table, |raw| {
            table_with_body_cell(vec![RenderNode::span(vec!["note".into()], raw)])
        }),
    ]
}

fn document(place: Place, fragments: &[&str]) -> RenderNode {
    RenderNode::root(vec![
        place(raws(fragments)),
        RenderNode::paragraph(vec![RenderNode::text("after")]),
    ])
}

fn line_ending_diagnostics(diagnostics: &[Diagnostic]) -> Vec<&Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.message.starts_with(LINE_ENDING))
        .collect()
}

/// The top-level block structure a GFM reader sees: each top-level block's
/// kind, with a table's cell count per row.
fn structure(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut nesting = 0usize;
    let mut row_cells: Option<usize> = None;
    for event in Parser::new_ext(markdown, Options::ENABLE_TABLES) {
        match event {
            Event::Start(tag) => {
                if nesting == 0 {
                    blocks.push(match &tag {
                        Tag::Heading { .. } => "heading".to_string(),
                        Tag::Paragraph => "paragraph".to_string(),
                        Tag::Table(_) => "table".to_string(),
                        Tag::HtmlBlock => "html block".to_string(),
                        other => format!("{other:?}"),
                    });
                }
                match &tag {
                    Tag::TableHead | Tag::TableRow => row_cells = Some(0),
                    Tag::TableCell => *row_cells.as_mut().expect("in a row") += 1,
                    _ => {}
                }
                nesting += 1;
            }
            Event::End(end) => {
                nesting -= 1;
                if matches!(end, TagEnd::TableHead | TagEnd::TableRow) {
                    let cells = row_cells.take().expect("in a row");
                    blocks.push(format!("row of {cells}"));
                }
            }
            _ => {}
        }
    }
    blocks
}

fn expected_structure(shape: Shape) -> Vec<String> {
    match shape {
        Shape::Heading => vec!["heading", "paragraph"],
        Shape::Table => vec!["table", "row of 2", "row of 2", "paragraph"],
    }
    .into_iter()
    .map(String::from)
    .collect()
}

#[test]
fn raw_line_endings_in_single_line_contexts_are_rejected_under_strict() {
    for dialect in DIALECTS {
        for (place_name, _, place) in placements() {
            for (payload_name, fragments) in payloads() {
                let case = format!("{dialect:?} / {place_name} / {payload_name}");
                let result = render_markdown_node(
                    &document(place, &fragments),
                    &opts(dialect, RenderStrictness::Strict),
                );
                assert!(
                    matches!(result, Err(RenderError::LossyRejected { .. })),
                    "{case}: {result:?}"
                );
            }
        }
    }
}

#[test]
fn raw_line_endings_in_single_line_contexts_are_encoded_and_reported_under_warn() {
    for dialect in DIALECTS {
        for (place_name, shape, place) in placements() {
            for (payload_name, fragments) in payloads() {
                let case = format!("{dialect:?} / {place_name} / {payload_name}");
                let rendered = render_markdown_node(
                    &document(place, &fragments),
                    &opts(dialect, RenderStrictness::Warn),
                )
                .unwrap_or_else(|error| panic!("{case}: {error}"));
                assert!(
                    rendered.output.contains(&encoded(&fragments)),
                    "{case}: {:?}",
                    rendered.output
                );
                assert_eq!(
                    structure(&rendered.output),
                    expected_structure(shape),
                    "{case}: {:?}",
                    rendered.output
                );
                let reported = line_ending_diagnostics(&rendered.diagnostics);
                assert_eq!(
                    reported.len(),
                    holding_endings(&fragments),
                    "{case}: {:?}",
                    rendered.diagnostics
                );
                let expected_spans: Vec<_> = fragments
                    .iter()
                    .enumerate()
                    .filter(|(_, fragment)| fragment.contains(['\r', '\n']))
                    .map(|(index, _)| Some(span_at(index)))
                    .collect();
                let spans: Vec<_> = reported.iter().map(|d| d.span.clone()).collect();
                assert_eq!(spans, expected_spans, "{case}");
            }
        }
    }
}

#[test]
fn raw_line_endings_in_single_line_contexts_are_encoded_silently_under_lossy() {
    for dialect in DIALECTS {
        for (place_name, shape, place) in placements() {
            for (payload_name, fragments) in payloads() {
                let case = format!("{dialect:?} / {place_name} / {payload_name}");
                let tree = document(place, &fragments);
                let lossy = render_markdown_node(&tree, &opts(dialect, RenderStrictness::Lossy))
                    .unwrap_or_else(|error| panic!("{case}: {error}"));
                let warn = render_markdown_node(&tree, &opts(dialect, RenderStrictness::Warn))
                    .unwrap_or_else(|error| panic!("{case}: {error}"));
                assert_eq!(lossy.output, warn.output, "{case}");
                assert_eq!(
                    structure(&lossy.output),
                    expected_structure(shape),
                    "{case}: {:?}",
                    lossy.output
                );
                assert!(
                    lossy.diagnostics.is_empty(),
                    "{case}: {:?}",
                    lossy.diagnostics
                );
            }
        }
    }
}

#[test]
fn raw_payloads_without_line_endings_stay_byte_identical_in_single_line_contexts() {
    let payload = "<span title=\"a b\">t</span>";
    for dialect in DIALECTS {
        for (place_name, shape, place) in placements() {
            for strictness in [RenderStrictness::Warn, RenderStrictness::Lossy] {
                let case = format!("{dialect:?} / {place_name} / {strictness:?}");
                let rendered =
                    render_markdown_node(&document(place, &[payload]), &opts(dialect, strictness))
                        .unwrap_or_else(|error| panic!("{case}: {error}"));
                assert!(
                    rendered.output.contains(payload),
                    "{case}: {:?}",
                    rendered.output
                );
                assert_eq!(
                    structure(&rendered.output),
                    expected_structure(shape),
                    "{case}"
                );
                assert!(
                    line_ending_diagnostics(&rendered.diagnostics).is_empty(),
                    "{case}: {:?}",
                    rendered.diagnostics
                );
                // MarkdownPlus permits raw HTML; only plain Markdown reports
                // it (as not portable) under Warn.
                if dialect == MarkdownDialect::MarkdownPlus {
                    assert!(rendered.diagnostics.is_empty(), "{case}");
                }
            }
        }
    }
}

#[test]
fn raw_payloads_without_line_endings_are_accepted_under_strict_markdown_plus() {
    let payload = "<span title=\"a b\">t</span>";
    for (place_name, _, place) in placements() {
        let rendered = render_markdown_node(
            &document(place, &[payload]),
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict),
        )
        .unwrap_or_else(|error| panic!("{place_name}: {error}"));
        assert!(rendered.output.contains(payload), "{place_name}");
        assert!(rendered.diagnostics.is_empty(), "{place_name}");
    }
}

#[test]
fn raw_line_endings_outside_single_line_contexts_stay_byte_identical() {
    for (payload_name, fragments) in payloads() {
        let tree = RenderNode::root(vec![RenderNode::paragraph(raws(&fragments))]);
        let rendered = render_markdown_node(
            &tree,
            &opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict),
        )
        .unwrap_or_else(|error| panic!("{payload_name}: {error}"));
        assert_eq!(rendered.output, fragments.concat(), "{payload_name}");
        assert!(rendered.diagnostics.is_empty(), "{payload_name}");
    }
}
