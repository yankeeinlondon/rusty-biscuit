//! Block content inside a GFM table cell.
//!
//! A GFM cell holds one line of inline content, so a block there (a code
//! block, list, heading, quote, columns, disclosure, rule, nested table,
//! footnote definition, or a second paragraph) has no faithful spelling. The
//! writer applies the strictness model: `Strict` rejects the tree, `Warn`
//! writes the block on the cell's line with one lossy diagnostic per cell,
//! and `Lossy` writes the same line silently. A cell holding exactly one
//! paragraph is that paragraph's text and stays clean.
//!
//! Each case is read back with an independent GFM reader (`pulldown-cmark`)
//! and checked for structure (every row keeps two cells, the neighboring
//! sentinels are intact) and for the degraded cell's value.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::tree::{
    ColumnAlign, ColumnsHints, DiagnosticKind, HeadingDepth, MarkdownDialect,
    MarkdownRenderOptions, ProgressHints, RenderError, RenderNode, RenderStrictness,
    render_markdown_node,
};

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

fn opts(dialect: MarkdownDialect, strictness: RenderStrictness) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        dialect,
        strictness,
        style: None,
    }
}

fn para(value: &str) -> RenderNode {
    RenderNode::paragraph(vec![RenderNode::text(value)])
}

fn depth() -> HeadingDepth {
    HeadingDepth::new(2).expect("valid depth")
}

/// Where the target cell sits: header or body row, first or second column.
const POSITIONS: [(bool, usize); 4] = [(true, 0), (true, 1), (false, 0), (false, 1)];

const SENTINELS: [[&str; 2]; 2] = [["H1", "H2"], ["y", "z"]];

/// A two-row, two-column table with `cell` at `position`.
fn table_with(position: (bool, usize), cell: Vec<RenderNode>) -> RenderNode {
    let (header, column) = position;
    let mut cell = Some(cell);
    let rows = (0..2)
        .map(|row| {
            RenderNode::table_row(
                (0..2)
                    .map(|col| {
                        let children = if (row == 0) == header && col == column {
                            cell.take().expect("one target cell")
                        } else {
                            vec![RenderNode::text(SENTINELS[row][col])]
                        };
                        RenderNode::table_cell(children)
                    })
                    .collect(),
            )
        })
        .collect();
    RenderNode::root(vec![RenderNode::table(
        vec![ColumnAlign::None, ColumnAlign::None],
        rows,
    )])
}

/// The cells of the document's only table, row by row. Inline events are
/// written as comparable strings; `<br>` stays as the reader's inline HTML.
fn read_table(markdown: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut tables = 0;
    let mut cell: Option<String> = None;
    let parser = Parser::new_ext(
        markdown,
        Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_STRIKETHROUGH,
    );
    for event in parser {
        match &event {
            Event::Start(Tag::Table(_)) => tables += 1,
            Event::Start(Tag::TableHead | Tag::TableRow) => rows.push(Vec::new()),
            Event::Start(Tag::TableCell) => cell = Some(String::new()),
            Event::End(TagEnd::TableCell) => {
                let value = cell.take().expect("cell open");
                rows.last_mut().expect("row open").push(value);
            }
            _ => {
                if let Some(value) = cell.as_mut() {
                    value.push_str(&match &event {
                        Event::Text(text) => text.to_string(),
                        Event::Code(code) => format!("`{code}`"),
                        Event::InlineHtml(html) => html.to_string(),
                        Event::Start(tag) => format!("<{tag:?}>"),
                        Event::End(tag) => format!("</{tag:?}>"),
                        other => format!("{other:?}"),
                    });
                }
            }
        }
    }
    assert_eq!(tables, 1, "expected one table in:\n{markdown}");
    rows
}

/// The table a reader should see with `target` at `position`.
fn wanted(position: (bool, usize), target: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = SENTINELS
        .iter()
        .map(|row| row.iter().map(|value| (*value).to_string()).collect())
        .collect();
    rows[usize::from(!position.0)][position.1] = target.to_string();
    rows
}

fn columns() -> RenderNode {
    let mut node = RenderNode::block_quote(vec![para("l|x"), para("r *s*")]);
    node.attrs.set_columns_hints(&ColumnsHints {
        left_count: 1,
        ..ColumnsHints::default()
    });
    node
}

/// Each block shape a cell can hold, with the value its one-line spelling
/// reads back as.
fn block_cases() -> Vec<(&'static str, Vec<RenderNode>, &'static str)> {
    vec![
        ("two paragraphs", vec![para("a|b"), para("c")], "a|b<br>c"),
        (
            "text around a paragraph",
            vec![RenderNode::text("t"), para("b"), RenderNode::text("u")],
            "t<br>b<br>u",
        ),
        (
            "code block",
            vec![RenderNode::code(
                Some("rust".into()),
                None,
                "let a = x || y;\nnext\n",
            )],
            "`let a = x || y; next`",
        ),
        (
            "list",
            vec![RenderNode::list(
                true,
                Some(3),
                vec![
                    RenderNode::list_item(Some(true), vec![para("one")]),
                    RenderNode::list_item(
                        None,
                        vec![
                            para("two"),
                            RenderNode::list(
                                false,
                                None,
                                vec![RenderNode::list_item(None, vec![para("n|m")])],
                            ),
                        ],
                    ),
                ],
            )],
            "3. [x] one<br>4. two<br>- n|m",
        ),
        (
            "heading",
            vec![RenderNode::heading(depth(), vec![RenderNode::text("T|t #")])],
            "T|t #",
        ),
        (
            "section",
            vec![RenderNode::section(
                depth(),
                vec![RenderNode::text("S")],
                vec![para("body")],
            )],
            "S<br>body",
        ),
        (
            "block quote",
            vec![RenderNode::block_quote(vec![para("q1"), para("q2")])],
            "q1<br>q2",
        ),
        ("columns", vec![columns()], "l|x<br>r *s*"),
        (
            "thematic break",
            vec![para("a"), RenderNode::thematic_break(), para("b")],
            "a<br><br>b",
        ),
        (
            "nested table",
            vec![RenderNode::table(
                vec![ColumnAlign::None],
                vec![
                    RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text(
                        "i1",
                    )])]),
                    RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text(
                        "i|2",
                    )])]),
                ],
            )],
            "i1<br>i|2",
        ),
        (
            "footnote definition",
            vec![RenderNode::footnote_definition("n", vec![para("note")])],
            "note",
        ),
        (
            "disclosure",
            vec![RenderNode::disclosure(
                vec![RenderNode::text("sum|mary")],
                vec![para("det"), RenderNode::code(None, None, "c")],
                None,
            )],
            "sum|mary<br>det<br>`c`",
        ),
        (
            "block inside an inline extension",
            vec![RenderNode::extended(
                "custom",
                vec![RenderNode::text("e"), para("inner")],
                None,
            )],
            "e<br>inner",
        ),
    ]
}

#[test]
fn a_block_in_a_cell_follows_the_strictness_model_and_keeps_the_row() {
    let mut failures = Vec::new();
    let mut cases = 0;
    for (name, cell, reading) in block_cases() {
        for dialect in DIALECTS {
            for position in POSITIONS {
                cases += 1;
                let tree = table_with(position, cell.clone());
                let case = format!("{name} / {dialect:?} / {position:?}");

                match render_markdown_node(&tree, &opts(dialect, RenderStrictness::Strict)) {
                    Err(RenderError::LossyRejected { message }) => {
                        if !message.contains("table cell") {
                            failures.push(format!("{case}: Strict message {message:?}"));
                        }
                    }
                    other => failures.push(format!("{case}: Strict gave {other:?}")),
                }

                let warn = render_markdown_node(&tree, &opts(dialect, RenderStrictness::Warn))
                    .expect("Warn renders");
                let lossy = render_markdown_node(&tree, &opts(dialect, RenderStrictness::Lossy))
                    .expect("Lossy renders");
                let cell_diagnostics: Vec<_> = warn
                    .diagnostics
                    .iter()
                    .filter(|d| d.message.contains("table cell"))
                    .collect();
                if cell_diagnostics.len() != 1
                    || cell_diagnostics[0].kind != DiagnosticKind::Lossy
                {
                    failures.push(format!("{case}: Warn diagnostics {:?}", warn.diagnostics));
                }
                if !lossy.diagnostics.is_empty() {
                    failures.push(format!("{case}: Lossy diagnostics {:?}", lossy.diagnostics));
                }
                if warn.output != lossy.output {
                    failures.push(format!(
                        "{case}: Warn and Lossy differ:\n{}\n{}",
                        warn.output, lossy.output
                    ));
                }
                if warn.output.lines().count() != 3 {
                    failures.push(format!("{case}: not three table lines:\n{}", warn.output));
                }
                let rows = read_table(&warn.output);
                if rows != wanted(position, reading) {
                    failures.push(format!("{case}: read {rows:?} from:\n{}", warn.output));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {cases} cases failed:\n{}",
        failures.len(),
        failures.join("\n---\n")
    );
}

#[test]
fn a_cell_holding_only_inline_content_or_one_paragraph_stays_clean() {
    let mut progress = RenderNode::paragraph(vec![RenderNode::text("Load 60%")]);
    progress.attrs.set_progress_hints(&ProgressHints {
        value: 0.6,
        ..ProgressHints::default()
    });
    let cells: Vec<(&str, Vec<RenderNode>, &str)> = vec![
        ("inline", vec![RenderNode::text("a|b")], "a|b"),
        ("one paragraph", vec![para("a|b")], "a|b"),
        (
            "one paragraph with a hard break",
            vec![RenderNode::paragraph(vec![
                RenderNode::text("a"),
                RenderNode::hard_break(),
                RenderNode::text("b"),
            ])],
            "a<br>b",
        ),
    ];
    for dialect in DIALECTS {
        let strict = opts(dialect, RenderStrictness::Strict);
        for (name, cell, reading) in &cells {
            for position in POSITIONS {
                let rendered = render_markdown_node(&table_with(position, cell.clone()), &strict)
                    .unwrap_or_else(|error| panic!("{name} {dialect:?}: {error}"));
                assert!(rendered.diagnostics.is_empty(), "{name}: {:?}", rendered.diagnostics);
                assert_eq!(
                    read_table(&rendered.output),
                    wanted(position, reading),
                    "{name} {dialect:?} {position:?}\n{}",
                    rendered.output
                );
            }
        }
        // A progress paragraph is one paragraph too.
        let rendered = render_markdown_node(&table_with(POSITIONS[2], vec![progress.clone()]), &strict)
            .expect("progress renders");
        assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
        let rows = read_table(&rendered.output);
        assert_eq!(rows[1].len(), 2, "{}", rendered.output);
        assert_eq!(rows[1][1], "z", "{}", rendered.output);
    }
}

#[test]
fn a_cell_writes_one_diagnostic_however_many_blocks_it_holds() {
    let cell = vec![para("a"), RenderNode::code(None, None, "b"), para("c")];
    let rendered = render_markdown_node(
        &table_with(POSITIONS[2], cell),
        &opts(MarkdownDialect::Markdown, RenderStrictness::Warn),
    )
    .expect("Warn renders");
    assert_eq!(rendered.diagnostics.len(), 1, "{:?}", rendered.diagnostics);
    assert!(
        rendered.diagnostics[0].message.starts_with("Paragraph block"),
        "{:?}",
        rendered.diagnostics
    );
    assert_eq!(
        read_table(&rendered.output),
        wanted(POSITIONS[2], "a<br>`b`<br>c")
    );
}

/// A heading is one line too. Validation keeps blocks out of a heading's
/// direct children, but an inline extension can carry one in; it gets the
/// same one-line spelling and strictness, with the heading's own escaping
/// (a pipe is not escaped there).
#[test]
fn a_block_carried_into_a_heading_by_an_extension_stays_on_the_heading_line() {
    let heading = RenderNode::root(vec![
        RenderNode::heading(
            depth(),
            vec![
                RenderNode::text("h"),
                RenderNode::extended(
                    "custom",
                    vec![RenderNode::code(None, None, "a || b\nc\n"), para("p")],
                    None,
                ),
            ],
        ),
        para("after"),
    ]);
    for dialect in DIALECTS {
        assert!(matches!(
            render_markdown_node(&heading, &opts(dialect, RenderStrictness::Strict)),
            Err(RenderError::LossyRejected { message }) if message.contains("Code block in a heading")
        ));
        let warn = render_markdown_node(&heading, &opts(dialect, RenderStrictness::Warn))
            .expect("Warn renders");
        assert_eq!(warn.diagnostics.len(), 1, "{:?}", warn.diagnostics);
        let lossy = render_markdown_node(&heading, &opts(dialect, RenderStrictness::Lossy))
            .expect("Lossy renders");
        assert!(lossy.diagnostics.is_empty(), "{:?}", lossy.diagnostics);
        assert_eq!(lossy.output, warn.output);

        let mut blocks = Vec::new();
        let mut current = String::new();
        for event in Parser::new_ext(&warn.output, Options::empty()) {
            match event {
                Event::Start(Tag::Heading { .. } | Tag::Paragraph) => current.clear(),
                Event::End(TagEnd::Heading(_)) => blocks.push(format!("h2:{current}")),
                Event::End(TagEnd::Paragraph) => blocks.push(format!("p:{current}")),
                Event::Text(text) | Event::InlineHtml(text) => current.push_str(&text),
                Event::Code(code) => current.push_str(&format!("`{code}`")),
                other => current.push_str(&format!("{other:?}")),
            }
        }
        assert_eq!(
            blocks,
            ["h2:h<br>`a || b c`<br>p", "p:after"],
            "{dialect:?}\n{}",
            warn.output
        );
    }
}
