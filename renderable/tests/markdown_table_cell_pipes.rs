//! A `|` written by any field inside a GFM table cell.
//!
//! GFM splits a table row into cells before it parses anything inline, so a
//! pipe is a cell delimiter wherever it sits: in text, in a code span, in a
//! raw HTML tag or comment, in a footnote identifier. Every field the writer
//! puts in a cell therefore has to make its pipes row-safe, and has to do it
//! without changing what the cell reads as.
//!
//! The rule under test: an authored payload (text, code, raw HTML, footnote
//! identifier, link and image fields) gets `\|` for each `|`, which the
//! table reader removes again before inline parsing; a generated attribute
//! value or other generated HTML gets `&#124;`, which an HTML reader decodes.
//!
//! Each case is read back with an independent GFM reader (`pulldown-cmark`
//! with tables and footnotes) and checked twice: the table keeps two cells in
//! every row, with the neighboring sentinel cells intact, and the target cell
//! reads exactly like the same content written in a paragraph outside a
//! table.
//!
//! pulldown-cmark keeps the cell's `\|` escape inside an inline HTML token,
//! where cmark-gfm (GitHub's reader) removes it as it does everywhere else
//! in the cell. The cell reading applies cmark-gfm's removal to those tokens
//! so both readings describe the same document.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::tree::{
    ColumnAlign, HeadingDepth, MarkdownDialect, MarkdownRenderOptions, ProgressHints, RenderError,
    RenderNode, RenderStrictness, render_markdown_node,
};

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

fn opts(dialect: MarkdownDialect, strictness: RenderStrictness) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        dialect,
        strictness,
        style: None,
    }
}

/// Raw payloads by name. The first seven hold an unescaped pipe in each
/// place raw HTML can carry one; the last three are controls whose pipe is
/// already escaped or written as a character reference.
const PAYLOADS: [(&str, &str); 10] = [
    ("span text", "<span>a|b</span>"),
    ("quoted attribute", "<span title=\"a|b\">t</span>"),
    ("unquoted attribute", "<span title=a|b>t</span>"),
    ("comment", "<!-- a|b -->"),
    ("script", "<script>a|b</script>"),
    ("style", "<style>a|b</style>"),
    ("code element", "<code>a|b</code>"),
    ("preescaped pipe", "<span>a\\|b</span>"),
    ("reference in text", "<span>a&#124;b</span>"),
    (
        "reference in quoted attribute",
        "<span title=\"a&#124;b\">t</span>",
    ),
];

/// Every way to cut `payload` into two raw nodes, empty fragments included,
/// plus the whole payload as one node.
fn splits(payload: &str) -> Vec<Vec<RenderNode>> {
    let mut all = vec![vec![RenderNode::html(payload, false)]];
    for at in 0..=payload.len() {
        all.push(vec![
            RenderNode::html(&payload[..at], false),
            RenderNode::html(&payload[at..], false),
        ]);
    }
    all
}

type Wrap = fn(Vec<RenderNode>) -> Vec<RenderNode>;

/// Every inline route raw HTML takes into a cell.
fn wrappers() -> Vec<(&'static str, Wrap)> {
    vec![
        ("direct", |nodes| nodes),
        ("strong", |nodes| vec![RenderNode::strong(nodes)]),
        ("emphasis", |nodes| vec![RenderNode::emphasis(nodes)]),
        ("delete", |nodes| vec![RenderNode::delete(nodes)]),
        ("link", |nodes| {
            vec![RenderNode::link("https://x.test", None, nodes)]
        }),
        ("neutral span", |nodes| {
            vec![RenderNode::span(vec![], nodes)]
        }),
        ("classed span", |nodes| {
            vec![RenderNode::span(vec!["note".into()], nodes)]
        }),
        ("unknown extension", |nodes| {
            vec![RenderNode::extended("custom", nodes, None)]
        }),
        ("mark", |nodes| {
            vec![RenderNode::extended("mark", nodes, None)]
        }),
        ("dim", |nodes| {
            vec![RenderNode::extended("dim", nodes, None)]
        }),
    ]
}

/// Where the target cell sits: header or body row, first or second column.
#[derive(Debug, Clone, Copy)]
struct Position {
    header: bool,
    column: usize,
}

const POSITIONS: [Position; 4] = [
    Position {
        header: true,
        column: 0,
    },
    Position {
        header: true,
        column: 1,
    },
    Position {
        header: false,
        column: 0,
    },
    Position {
        header: false,
        column: 1,
    },
];

/// The sentinel values of the four cells; the target replaces one.
const SENTINELS: [[&str; 2]; 2] = [["H1", "H2"], ["y", "z"]];

/// A two-row, two-column table with `cell` at `position`, followed by
/// `after` (a footnote definition, for example).
fn table_with(position: Position, cell: Vec<RenderNode>, after: Vec<RenderNode>) -> RenderNode {
    let mut cell = Some(cell);
    let rows = (0..2)
        .map(|row| {
            RenderNode::table_row(
                (0..2)
                    .map(|column| {
                        let target = (row == 0) == position.header && column == position.column;
                        let children = if target {
                            cell.take().expect("one target cell")
                        } else {
                            vec![RenderNode::text(SENTINELS[row][column])]
                        };
                        RenderNode::table_cell(children)
                    })
                    .collect(),
            )
        })
        .collect();
    let mut blocks = vec![RenderNode::table(
        vec![ColumnAlign::None, ColumnAlign::None],
        rows,
    )];
    blocks.extend(after);
    RenderNode::root(blocks)
}

fn depth() -> HeadingDepth {
    HeadingDepth::new(2).expect("valid depth")
}

fn parse(markdown: &str) -> Vec<Event<'_>> {
    Parser::new_ext(
        markdown,
        Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_STRIKETHROUGH,
    )
    .collect()
}

/// An inline event as a comparable string. In a table cell an inline HTML
/// token has cmark-gfm's pipe-escape removal applied (see the module docs).
fn describe(event: &Event<'_>, in_cell: bool) -> String {
    match event {
        Event::Text(text) => text.to_string(),
        Event::Code(code) => format!("`{code}`"),
        Event::InlineHtml(html) | Event::Html(html) if in_cell => html.replace("\\|", "|"),
        Event::InlineHtml(html) | Event::Html(html) => html.to_string(),
        Event::FootnoteReference(label) => format!("[^{label}]"),
        Event::SoftBreak => "\n".into(),
        Event::HardBreak => "<hard>".into(),
        Event::Start(Tag::Link {
            dest_url, title, ..
        }) => format!("<link \"{dest_url}\" \"{title}\">"),
        Event::Start(Tag::Image {
            dest_url, title, ..
        }) => format!("<image \"{dest_url}\" \"{title}\">"),
        Event::Start(tag) => format!("<{tag:?}>"),
        Event::End(tag) => format!("</{tag:?}>"),
        other => format!("{other:?}"),
    }
}

/// The table's cells as read back, row by row, and every footnote
/// definition label in the document. Panics unless the document holds
/// exactly one table.
fn read_table(markdown: &str) -> (Vec<Vec<String>>, Vec<String>) {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut definitions = Vec::new();
    let mut tables = 0;
    let mut cell: Option<String> = None;
    for event in parse(markdown) {
        match &event {
            Event::Start(Tag::Table(_)) => tables += 1,
            Event::Start(Tag::TableHead | Tag::TableRow) => rows.push(Vec::new()),
            Event::Start(Tag::TableCell) => cell = Some(String::new()),
            Event::End(TagEnd::TableCell) => {
                let value = cell.take().expect("cell open");
                rows.last_mut().expect("row open").push(value);
            }
            Event::Start(Tag::FootnoteDefinition(label)) => definitions.push(label.to_string()),
            _ => {
                if let Some(value) = cell.as_mut() {
                    value.push_str(&describe(&event, true));
                }
            }
        }
    }
    assert_eq!(tables, 1, "expected one table in:\n{markdown}");
    (rows, definitions)
}

/// How `children` read in a paragraph outside any table, after a leading
/// text value that keeps them mid-line, as they are in a cell.
fn read_outside_table(children: &[RenderNode], options: &MarkdownRenderOptions) -> String {
    let mut inline = vec![RenderNode::text("p ")];
    inline.extend(children.iter().cloned());
    let markdown = render_markdown_node(
        &RenderNode::root(vec![RenderNode::paragraph(inline)]),
        options,
    )
    .expect("paragraph renders")
    .output;
    let read: String = parse(&markdown)
        .iter()
        .filter(|event| {
            !matches!(
                event,
                Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph)
            )
        })
        .map(|event| describe(event, false))
        .collect();
    read.strip_prefix("p ")
        .unwrap_or_else(|| panic!("paragraph reading lost its prefix: {read:?}"))
        .to_string()
}

/// Renders `cell` at `position` and checks the reader's table: two cells per
/// row, every sentinel intact, and the target cell reading as `children` do
/// outside a table. Returns a failure description instead of panicking so a
/// matrix can report every failing case at once.
fn check_cell(
    position: Position,
    children: Vec<RenderNode>,
    options: &MarkdownRenderOptions,
) -> Result<(), String> {
    let expected = read_outside_table(&children, options);
    let markdown = render_markdown_node(&table_with(position, children, vec![]), options)
        .map_err(|error| format!("render failed: {error}"))?
        .output;
    let (rows, _) = std::panic::catch_unwind(|| read_table(&markdown))
        .map_err(|_| format!("no table in:\n{markdown}"))?;
    let row = usize::from(!position.header);
    let mut wanted: Vec<Vec<String>> = SENTINELS
        .iter()
        .map(|row| row.iter().map(|value| (*value).to_string()).collect())
        .collect();
    wanted[row][position.column] = expected;
    if rows == wanted {
        Ok(())
    } else {
        Err(format!(
            "read {rows:?}, wanted {wanted:?}, from:\n{markdown}"
        ))
    }
}

fn assert_no_failures(failures: &[String], cases: usize) {
    assert!(
        failures.is_empty(),
        "{} of {cases} cases failed; first ones:\n{}",
        failures.len(),
        failures
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n---\n")
    );
}

#[test]
fn raw_html_with_a_pipe_keeps_the_row_and_its_value_at_every_split_route_and_position() {
    let mut failures = Vec::new();
    let mut cases = 0;
    for dialect in DIALECTS {
        // Lossy: plain Markdown otherwise rejects or reports raw HTML for
        // portability, which is a separate policy (see the strictness test).
        let options = opts(dialect, RenderStrictness::Lossy);
        for (payload_name, payload) in PAYLOADS {
            for fragments in splits(payload) {
                for (wrapper_name, wrap) in wrappers() {
                    for position in POSITIONS {
                        cases += 1;
                        if let Err(failure) =
                            check_cell(position, wrap(fragments.clone()), &options)
                        {
                            failures.push(format!(
                                "{dialect:?} / {payload_name} / {wrapper_name} / {position:?} / \
                                 {fragments:?}: {failure}"
                            ));
                        }
                    }
                }
            }
        }
    }
    assert_no_failures(&failures, cases);
}

#[test]
fn raw_html_with_a_pipe_in_a_cell_is_accepted_without_a_cell_diagnostic() {
    let cell = vec![RenderNode::html("<span title=\"a|b\">t</span>", false)];
    for strictness in [
        RenderStrictness::Strict,
        RenderStrictness::Warn,
        RenderStrictness::Lossy,
    ] {
        let options = opts(MarkdownDialect::MarkdownPlus, strictness);
        let rendered =
            render_markdown_node(&table_with(POSITIONS[2], cell.clone(), vec![]), &options)
                .expect("MarkdownPlus keeps raw HTML in a cell");
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
        assert!(
            rendered
                .output
                .contains("| <span title=\"a\\|b\">t</span> | z |"),
            "{}",
            rendered.output
        );
    }

    // Plain Markdown keeps its raw-HTML portability policy; the cell escape
    // applies to what Warn and Lossy write.
    let strict = opts(MarkdownDialect::Markdown, RenderStrictness::Strict);
    assert!(matches!(
        render_markdown_node(&table_with(POSITIONS[2], cell.clone(), vec![]), &strict),
        Err(RenderError::LossyRejected { .. })
    ));
    let warn = opts(MarkdownDialect::Markdown, RenderStrictness::Warn);
    let rendered = render_markdown_node(&table_with(POSITIONS[2], cell.clone(), vec![]), &warn)
        .expect("Warn renders");
    assert_eq!(rendered.diagnostics.len(), 1, "{:?}", rendered.diagnostics);
    assert!(
        rendered.diagnostics[0]
            .message
            .contains("raw HTML is not portable"),
        "{:?}",
        rendered.diagnostics
    );
    assert_eq!(check_cell(POSITIONS[2], cell, &warn), Ok(()));
}

#[test]
fn a_raw_line_ending_and_a_pipe_in_one_cell_payload_are_both_made_row_safe() {
    let options = opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Lossy);
    let cell = vec![RenderNode::html("<span title=\"a|b\nc\">t</span>", false)];
    let markdown = render_markdown_node(&table_with(POSITIONS[3], cell, vec![]), &options)
        .expect("Lossy renders")
        .output;
    assert!(
        markdown.contains("| y | <span title=\"a\\|b&#10;c\">t</span> |"),
        "{markdown}"
    );
    let (rows, _) = read_table(&markdown);
    assert_eq!(rows[1].len(), 2, "{markdown}");
    assert_eq!(rows[1][0], "y");
}

#[test]
fn a_class_with_a_pipe_uses_a_reference_in_markdown_plus_and_is_dropped_in_markdown() {
    for position in POSITIONS {
        let cell = vec![RenderNode::span(
            vec!["a|b".into()],
            vec![RenderNode::text("t")],
        )];

        let plus = opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict);
        let rendered = render_markdown_node(&table_with(position, cell.clone(), vec![]), &plus)
            .expect("MarkdownPlus renders a classed span");
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
        assert!(
            rendered
                .output
                .contains("<span class=\"a&#124;b\">t</span>"),
            "{}",
            rendered.output
        );
        // The attribute spelling needs no reader-specific normalization.
        let (rows, _) = read_table(&rendered.output);
        let row = usize::from(!position.header);
        assert_eq!(
            rows[row][position.column], "<span class=\"a&#124;b\">t</span>",
            "{position:?}"
        );
        // An HTML reader decodes the reference back to the class's pipe.
        assert_eq!(
            rows[row][position.column].replace("&#124;", "|"),
            read_outside_table(&cell, &plus),
            "{position:?}"
        );
        assert_eq!(
            rows[1 - row][position.column],
            SENTINELS[1 - row][position.column]
        );
        assert_eq!(
            rows[row][1 - position.column],
            SENTINELS[row][1 - position.column]
        );

        // Plain Markdown keeps its existing degradation: the class is
        // removed (Lossy) and the text stays in its cell.
        let lossy = opts(MarkdownDialect::Markdown, RenderStrictness::Lossy);
        let output = render_markdown_node(&table_with(position, cell.clone(), vec![]), &lossy)
            .expect("Lossy renders")
            .output;
        assert!(!output.contains("class"), "{output}");
        assert_eq!(check_cell(position, cell.clone(), &lossy), Ok(()));
        let strict = opts(MarkdownDialect::Markdown, RenderStrictness::Strict);
        assert!(matches!(
            render_markdown_node(&table_with(position, cell, vec![]), &strict),
            Err(RenderError::LossyRejected { .. })
        ));
    }
}

#[test]
fn a_class_is_attribute_escaped_in_every_context() {
    let options = opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict);
    let span = RenderNode::span(vec!["a\"b&c<d'e".into()], vec![RenderNode::text("t")]);
    let output = render_markdown_node(
        &RenderNode::root(vec![RenderNode::paragraph(vec![span])]),
        &options,
    )
    .expect("renders")
    .output;
    assert_eq!(output, "<span class=\"a&quot;b&amp;c&lt;d&#39;e\">t</span>");
    // Outside a table a pipe is ordinary attribute content.
    let span = RenderNode::span(vec!["a|b".into()], vec![RenderNode::text("t")]);
    let output = render_markdown_node(
        &RenderNode::root(vec![RenderNode::paragraph(vec![span])]),
        &options,
    )
    .expect("renders")
    .output;
    assert_eq!(output, "<span class=\"a|b\">t</span>");
}

#[test]
fn a_footnote_identifier_with_a_pipe_still_resolves_from_a_cell() {
    for dialect in DIALECTS {
        for strictness in [
            RenderStrictness::Strict,
            RenderStrictness::Warn,
            RenderStrictness::Lossy,
        ] {
            let options = opts(dialect, strictness);
            for position in POSITIONS {
                let cell = vec![RenderNode::footnote_reference("a|b")];
                let definition =
                    RenderNode::footnote_definition("a|b", vec![RenderNode::text("note")]);
                let rendered =
                    render_markdown_node(&table_with(position, cell, vec![definition]), &options)
                        .expect("a footnote reference renders");
                assert!(
                    rendered.diagnostics.is_empty(),
                    "{:?}",
                    rendered.diagnostics
                );
                assert!(rendered.output.contains("[^a\\|b]"), "{}", rendered.output);
                // The definition is outside the table and keeps its pipe.
                assert!(
                    rendered.output.contains("\n[^a|b]: note"),
                    "{}",
                    rendered.output
                );
                let (rows, definitions) = read_table(&rendered.output);
                let mut wanted: Vec<Vec<String>> = SENTINELS
                    .iter()
                    .map(|row| row.iter().map(|value| (*value).to_string()).collect())
                    .collect();
                wanted[usize::from(!position.header)][position.column] = "[^a|b]".into();
                assert_eq!(
                    rows, wanted,
                    "{dialect:?} {position:?}\n{}",
                    rendered.output
                );
                assert_eq!(definitions, ["a|b"]);
            }
        }
    }
}

#[test]
fn text_code_link_and_image_fields_with_a_pipe_stay_clean() {
    let cells: Vec<(&str, Vec<RenderNode>)> = vec![
        ("text", vec![RenderNode::text("a|b")]),
        ("text with backslash", vec![RenderNode::text("a\\|b")]),
        ("code", vec![RenderNode::inline_code("a|b")]),
        (
            "code with backslash",
            vec![RenderNode::inline_code("a\\|b")],
        ),
        (
            "link label",
            vec![RenderNode::link(
                "https://x.test",
                None,
                vec![RenderNode::text("a|b")],
            )],
        ),
        (
            "link destination and title",
            vec![RenderNode::link(
                "https://x.test/a|b",
                Some("t|u".into()),
                vec![RenderNode::text("l")],
            )],
        ),
        (
            "image",
            vec![RenderNode::image(
                "https://x.test/a|b.png",
                Some("t|u".into()),
                "a|b",
            )],
        ),
    ];
    for dialect in DIALECTS {
        let options = opts(dialect, RenderStrictness::Strict);
        for (name, cell) in &cells {
            for position in POSITIONS {
                assert_eq!(
                    check_cell(position, cell.clone(), &options),
                    Ok(()),
                    "{dialect:?} {name} {position:?}"
                );
            }
        }
    }
}

#[test]
fn generated_progress_html_in_a_cell_writes_a_pipe_as_a_reference() {
    let options = opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Strict);

    let mut progress = RenderNode::paragraph(vec![RenderNode::text("Lo|ad 60%")]);
    progress.attrs.set_progress_hints(&ProgressHints {
        value: 0.6,
        fill_char: '|',
        ..ProgressHints::default()
    });
    let output = render_markdown_node(&table_with(POSITIONS[2], vec![progress], vec![]), &options)
        .expect("progress renders")
        .output;
    assert!(output.contains("aria-label=\"Lo&#124;ad\""), "{output}");
    assert!(output.contains("data-fill-char=\"&#124;\""), "{output}");
    assert!(output.contains(">Lo&#124;ad</span>"), "{output}");
    let (rows, _) = read_table(&output);
    assert_eq!(rows[1].len(), 2, "{output}");
    assert_eq!(rows[1][1], "z", "{output}");
    // Block HTML (columns, disclosure) is not written in a cell: a cell
    // writes a block on its line instead (see `markdown_table_cell_blocks`).
}

#[test]
fn an_unsupported_placeholder_label_with_a_pipe_keeps_the_row() {
    let options = opts(MarkdownDialect::MarkdownPlus, RenderStrictness::Warn);
    let cell = vec![RenderNode::unsupported("u|v")];
    let output = render_markdown_node(&table_with(POSITIONS[2], cell, vec![]), &options)
        .expect("Warn renders a placeholder")
        .output;
    assert!(output.contains("<!-- unsupported: u\\|v -->"), "{output}");
    let (rows, _) = read_table(&output);
    assert_eq!(
        rows[1],
        ["<!-- unsupported: u|v -->".to_string(), "z".to_string()],
        "{output}"
    );
}

#[test]
fn pipes_outside_a_table_keep_their_ordinary_meaning() {
    type Place = fn(Vec<RenderNode>) -> RenderNode;
    let places: Vec<(&str, Place)> = vec![
        ("heading", |raw| RenderNode::heading(depth(), raw)),
        ("section heading", |raw| {
            RenderNode::section(depth(), raw, vec![])
        }),
        ("heading > strong", |raw| {
            RenderNode::heading(depth(), vec![RenderNode::strong(raw)])
        }),
        ("heading > link", |raw| {
            RenderNode::heading(depth(), vec![RenderNode::link("https://x.test", None, raw)])
        }),
        ("heading > classed span", |raw| {
            RenderNode::heading(depth(), vec![RenderNode::span(vec!["note".into()], raw)])
        }),
        ("paragraph", |raw| {
            let mut inline = vec![RenderNode::text("p ")];
            inline.extend(raw);
            RenderNode::paragraph(inline)
        }),
    ];
    for dialect in DIALECTS {
        let options = opts(dialect, RenderStrictness::Lossy);
        for (payload_name, payload) in PAYLOADS {
            for fragments in splits(payload) {
                for (place_name, place) in &places {
                    let markdown = render_markdown_node(
                        &RenderNode::root(vec![place(fragments.clone())]),
                        &options,
                    )
                    .expect("renders")
                    .output;
                    assert!(
                        markdown.contains(payload),
                        "{dialect:?} {payload_name} {place_name} {fragments:?}:\n{markdown}"
                    );
                    assert!(
                        !parse(&markdown)
                            .iter()
                            .any(|event| matches!(event, Event::Start(Tag::Table(_)))),
                        "{markdown}"
                    );
                }
            }
        }
    }

    // A footnote reference outside a table keeps its identifier as is.
    let options = opts(MarkdownDialect::Markdown, RenderStrictness::Strict);
    let output = render_markdown_node(
        &RenderNode::root(vec![RenderNode::paragraph(vec![
            RenderNode::footnote_reference("a|b"),
        ])]),
        &options,
    )
    .expect("renders")
    .output;
    assert_eq!(output, "[^a|b]");
}
