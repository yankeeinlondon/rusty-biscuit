//! Neighboring code spans keep their values.
//!
//! A code span's fence is a backtick run that no other backtick touches, so
//! two fences written side by side become one longer run: `` `a``b` `` reads
//! as the single value ```` a``b ````. The writer must keep a boundary
//! between two code spans that meet, whether they are direct siblings or
//! become neighbors after an empty node, a flattened wrapper (neutral span,
//! unknown extension, a class or style removed in plain Markdown), or a
//! wrapper written unstyled.
//!
//! Each case places two code values, joined by one boundary shape and
//! wrapped by one of the table-cell regressions' inline routes, in one of
//! sixteen contexts, and renders it in both dialects under all three
//! strictness modes. Every result that renders is read with an independent
//! GFM reader (`pulldown-cmark`): the decoded code values, the visible text,
//! the wrapper and link extent around each value, the table's rows and
//! sentinel cells, and the heading or container the values sit in. No mode
//! may change a code value; a mode that rejects a case must be the one whose
//! Warn render reports a diagnostic.

#[path = "support/adjacent_code_matrix.rs"]
mod matrix;

use matrix::{
    Boundary, Context, DIALECTS, POSITIONS, SENTINELS, STRICTNESS, Tagged, contexts, for_each_case,
    groups, opts, table_with,
};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::tree::{MarkdownDialect, RenderError, RenderNode, RenderStrictness, render_markdown_node};

/// One code value as read, with the tags open around it.
#[derive(Debug, Clone, Default)]
struct Code {
    value: String,
    /// Read from a `<code>` element in raw HTML rather than a code span.
    html: bool,
    tags: Vec<Tagged>,
}

impl Code {
    fn count(&self, tag: Tagged) -> usize {
        self.tags.iter().filter(|t| **t == tag).count()
    }
}

/// A top-level block or a table cell.
#[derive(Debug, Default)]
struct Unit {
    kind: &'static str,
    text: String,
    codes: Vec<Code>,
}

#[derive(Debug, Default)]
struct Reading {
    tables: usize,
    blocks: Vec<Unit>,
    rows: Vec<Vec<Unit>>,
}

impl Reading {
    fn codes(&self) -> Vec<&Code> {
        self.blocks.iter().flat_map(|block| block.codes.iter()).collect()
    }
}

fn decode_html(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#96;", "`")
        .replace("&#x60;", "`")
        .replace("&#32;", " ")
        .replace("&amp;", "&")
}

/// Every `<code>` element's decoded content in `html`.
fn html_codes(html: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find("<code") {
        rest = &rest[at..];
        let open_end = rest.find('>').expect("open tag ends");
        rest = &rest[open_end + 1..];
        let close = rest.find("</code>").expect("code element closes");
        values.push(decode_html(&rest[..close]));
        rest = &rest[close..];
    }
    values
}

fn read(markdown: &str) -> Reading {
    let mut reading = Reading::default();
    let mut depth = 0usize;
    let mut tags: Vec<Tagged> = Vec::new();
    let mut cell: Option<Unit> = None;
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_STRIKETHROUGH;
    for event in Parser::new_ext(markdown, options) {
        let mut codes: Vec<Code> = Vec::new();
        let mut text = String::new();
        match &event {
            Event::Start(tag) => {
                if depth == 0 {
                    let kind = match tag {
                        Tag::Heading { .. } => "heading",
                        Tag::Paragraph => "paragraph",
                        Tag::Table(_) => "table",
                        Tag::HtmlBlock => "html",
                        Tag::BlockQuote(_) => "quote",
                        Tag::List(_) => "list",
                        Tag::FootnoteDefinition(_) => "footnote",
                        _ => "other",
                    };
                    if kind == "table" {
                        reading.tables += 1;
                    }
                    reading.blocks.push(Unit {
                        kind,
                        ..Unit::default()
                    });
                }
                depth += 1;
                match tag {
                    Tag::TableHead | Tag::TableRow => reading.rows.push(Vec::new()),
                    Tag::TableCell => cell = Some(Unit::default()),
                    Tag::Strong => tags.push(Tagged::Strong),
                    Tag::Emphasis => tags.push(Tagged::Emphasis),
                    Tag::Strikethrough => tags.push(Tagged::Delete),
                    Tag::Link { .. } => tags.push(Tagged::Link),
                    _ => {}
                }
            }
            Event::End(tag) => {
                depth -= 1;
                match tag {
                    TagEnd::TableCell => {
                        let value = cell.take().expect("cell open");
                        reading.rows.last_mut().expect("row open").push(value);
                    }
                    TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough | TagEnd::Link => {
                        tags.pop();
                    }
                    _ => {}
                }
            }
            Event::Text(value) => text.push_str(value),
            Event::SoftBreak => text.push(' '),
            Event::HardBreak => text.push('\n'),
            Event::Code(value) => {
                text.push_str(value);
                codes.push(Code {
                    value: value.to_string(),
                    html: false,
                    tags: tags.clone(),
                });
            }
            Event::InlineHtml(html) | Event::Html(html) => {
                // The fallback spellings of emphasis, strong, and delete.
                for (open, close, tag) in [
                    ("<em>", "</em>", Tagged::Emphasis),
                    ("<strong>", "</strong>", Tagged::Strong),
                    ("<del>", "</del>", Tagged::Delete),
                ] {
                    if html.as_ref() == open {
                        tags.push(tag);
                    } else if html.as_ref() == close {
                        tags.pop();
                    }
                }
                codes.extend(html_codes(html).into_iter().map(|value| Code {
                    value,
                    html: true,
                    tags: Vec::new(),
                }));
            }
            _ => {}
        }
        for unit in cell.iter_mut().chain(reading.blocks.last_mut()) {
            unit.text.push_str(&text);
            unit.codes.extend(codes.iter().cloned());
        }
    }
    reading
}

/// Everything wrong with how `markdown` carries the case.
fn check(
    markdown: &str,
    context: Context,
    (a, b): (&str, &str),
    boundary: &Boundary,
    route_tag: Option<Tagged>,
    html: bool,
) -> Vec<String> {
    let reading = read(markdown);
    let mut problems = Vec::new();
    let expected: Vec<&str> = [a, b].into_iter().filter(|v| !v.is_empty()).collect();
    let codes: Vec<&Code> = reading
        .codes()
        .into_iter()
        .filter(|code| !code.value.is_empty())
        .collect();
    let values: Vec<&str> = codes.iter().map(|code| code.value.as_str()).collect();
    if values != expected {
        problems.push(format!("code values {values:?}, expected {expected:?}"));
        return problems;
    }
    // MarkdownPlus lowers a disclosure summary and columns to HTML, where
    // each value is its own `<code>` element; everywhere else each value is
    // a code span.
    if codes.iter().any(|code| code.html != html) {
        problems.push(format!("expected every value from HTML {html}: {codes:?}"));
    }
    if !html {
        if let Some(tag) = route_tag
            && codes.iter().any(|code| code.count(tag) == 0)
        {
            problems.push(format!("a value is outside the {tag:?} route: {codes:?}"));
        }
        if let (Some(tag), [first, second]) = (boundary.second_in, codes.as_slice())
            && second.count(tag) != first.count(tag) + 1
        {
            problems.push(format!("{tag:?} does not wrap only the second: {codes:?}"));
        }
    }

    let target = match context {
        Context::Cell(position) => {
            if reading.tables != 1 || reading.blocks.len() != 1 {
                problems.push(format!("blocks {:?}", reading.blocks));
                return problems;
            }
            if reading.rows.len() != 2 || reading.rows.iter().any(|row| row.len() != 2) {
                problems.push(format!("rows {:?}", reading.rows));
                return problems;
            }
            let mut target = None;
            for (row, cells) in reading.rows.iter().enumerate() {
                for (column, cell) in cells.iter().enumerate() {
                    if (row == 0) == position.header && column == position.column {
                        target = Some(cell);
                    } else if cell.text != SENTINELS[row][column] || !cell.codes.is_empty() {
                        problems.push(format!("sentinel {row},{column} read {cell:?}"));
                    }
                }
            }
            target.expect("target cell")
        }
        _ => {
            let kinds: Vec<&str> = reading.blocks.iter().map(|block| block.kind).collect();
            let want: &[&str] = match context {
                Context::Paragraph | Context::Quote => &[
                    if context == Context::Quote { "quote" } else { "paragraph" },
                    "paragraph",
                ],
                Context::Heading | Context::SectionHeading => &["heading", "paragraph"],
                Context::Composition | Context::RootGrouped => &["paragraph"],
                Context::ListItem => &["list"],
                Context::Footnote => &["paragraph", "footnote"],
                // Disclosure and columns syntax is darkmatter's (plain) or an
                // HTML block (MarkdownPlus); its values are checked above.
                _ => &[],
            };
            if !want.is_empty() && kinds != want {
                problems.push(format!("blocks {kinds:?}, expected {want:?}"));
                return problems;
            }
            if want.len() == 2 && want[1] == "paragraph" && reading.blocks[1].text != "after" {
                problems.push(format!("after paragraph read {:?}", reading.blocks[1]));
            }
            match reading.blocks.iter().find(|block| !block.codes.is_empty()) {
                Some(block) => block,
                None => return problems,
            }
        }
    };
    // Both values sit in the expected block: the heading, the quote, the
    // footnote definition, and so on.
    if !matches!(context, Context::Cell(_)) && !layout_unmodeled(context) {
        let holder = if context == Context::Footnote { 1 } else { 0 };
        if reading.blocks[holder].codes.len() != reading.codes().len() {
            problems.push(format!("values outside block {holder}: {:?}", reading.blocks));
        }
    }
    // An ordinary root writes each inline child as its own block.
    if !html && context != Context::RootSeparate {
        let visible = format!("{a}{}{b}{}", boundary.between, boundary.after);
        // `==` and `⌄` are darkmatter syntax, plain text to a GFM reader, and
        // edge whitespace is written outside them.
        let read_text = target.text.replace("==", "").replace('\u{2304}', "");
        if !read_text.contains(&visible) {
            problems.push(format!("visible {visible:?} missing from {:?}", target.text));
        }
    }
    problems
}

/// Contexts whose block layout the reader does not model.
fn layout_unmodeled(context: Context) -> bool {
    matches!(
        context,
        Context::RootSeparate
            | Context::DisclosureSummary
            | Context::DisclosureBody
            | Context::Columns
    )
}

/// Renders every case in `contexts` under every dialect and strictness and
/// returns the failures.
fn run_matrix(contexts: &[Context]) -> (usize, Vec<String>) {
    let mut cases = 0;
    let mut failures = Vec::new();
    for_each_case(contexts, |case| {
        let (a, b) = case.values;
        for dialect in DIALECTS {
            let label = format!(
                "{:?} / {} / {} / {} / {dialect:?}",
                case.context, case.route, case.boundary.name, case.pair
            );
            let mut warned = None;
            let mut rejected = false;
            for strictness in STRICTNESS {
                cases += 1;
                match render_markdown_node(&case.node, &opts(dialect, strictness)) {
                    Ok(rendered) => {
                        if strictness == RenderStrictness::Warn {
                            warned = Some(rendered.diagnostics.len());
                        }
                        for problem in check(
                            &rendered.output,
                            case.context,
                            (a, b),
                            case.boundary,
                            case.route_tag,
                            dialect == MarkdownDialect::MarkdownPlus && case.context.html_in_markdown_plus(),
                        ) {
                            failures.push(format!("{label} / {strictness:?}: {problem}\n{}", rendered.output));
                        }
                    }
                    Err(RenderError::LossyRejected { .. }) if strictness == RenderStrictness::Strict => {
                        rejected = true;
                    }
                    Err(error) => {
                        failures.push(format!("{label} / {strictness:?}: {error}"));
                    }
                }
            }
            // Strict rejects exactly what Warn reports.
            if let Some(count) = warned
                && rejected != (count > 0)
            {
                failures.push(format!("{label}: Strict rejected {rejected}, Warn diagnostics {count}"));
            }
        }
    });
    (cases, failures)
}

fn assert_matrix(contexts: &[Context]) {
    let (cases, failures) = run_matrix(contexts);
    assert!(
        failures.is_empty(),
        "{} failures in {cases} cases; first ten:\n{}",
        failures.len(),
        failures.iter().take(4).cloned().collect::<Vec<_>>().join("\n---\n")
    );
}

#[test]
fn neighboring_code_values_survive_in_paragraphs_compositions_and_roots() {
    assert_matrix(&groups()[0]);
}

#[test]
fn neighboring_code_values_survive_in_headings_and_section_headings() {
    assert_matrix(&groups()[1]);
}

#[test]
fn neighboring_code_values_survive_in_every_table_cell_position() {
    assert_matrix(&groups()[2]);
}

#[test]
fn neighboring_code_values_survive_in_quotes_lists_and_footnotes() {
    assert_matrix(&groups()[3]);
}

#[test]
fn neighboring_code_values_survive_in_disclosures_and_columns() {
    assert_matrix(&groups()[4]);
}

#[test]
fn matrix_tests_cover_every_context() {
    let grouped: Vec<Context> = groups().into_iter().flatten().collect();
    assert_eq!(grouped, contexts());
    assert_eq!(grouped.len(), 16);
}

/// Direct neighbors need no diagnostic in any mode: the boundary is
/// lossless, and the separator is an empty HTML comment that both dialects
/// may write.
#[test]
fn direct_neighbors_render_without_diagnostics_in_every_mode() {
    let node = RenderNode::paragraph(vec![
        RenderNode::inline_code("a"),
        RenderNode::inline_code("b"),
    ]);
    for dialect in DIALECTS {
        for strictness in STRICTNESS {
            let rendered = render_markdown_node(&node, &opts(dialect, strictness))
                .unwrap_or_else(|error| panic!("{dialect:?} {strictness:?}: {error}"));
            assert_eq!(rendered.output, "`a`<!-- -->`b`", "{dialect:?} {strictness:?}");
            assert!(rendered.diagnostics.is_empty(), "{dialect:?} {strictness:?}");
        }
    }
}

/// The separator goes in only where two fences would touch: a visible
/// separator, a wrapper's own delimiter, or an empty value leaves the output
/// unchanged.
#[test]
fn separator_is_written_only_between_touching_fences() {
    let code = RenderNode::inline_code;
    let cases = [
        (vec![code("a"), RenderNode::text(" "), code("b")], "`a` `b`"),
        (vec![code("a"), RenderNode::strong(vec![code("b")])], "`a`**`b`**"),
        (vec![code("a"), code("")], "`a`"),
        (vec![code(""), code("b")], "`b`"),
        (
            vec![code("a"), code(""), code("b")],
            "`a`<!-- -->`b`",
        ),
        (
            vec![code("x`y"), code("z``w")],
            "``x`y``<!-- -->```z``w```",
        ),
    ];
    for (inline, expected) in cases {
        let node = RenderNode::paragraph(inline);
        for dialect in DIALECTS {
            let rendered =
                render_markdown_node(&node, &opts(dialect, RenderStrictness::Strict)).expect("render");
            assert_eq!(rendered.output, expected, "{dialect:?}");
        }
    }
}

/// A code block written on a table cell's line is still set off by `<br>`,
/// so it needs no separator from neighboring code values.
#[test]
fn block_code_in_a_cell_keeps_its_br_separators() {
    let cell = vec![
        RenderNode::inline_code("a"),
        RenderNode::extended(
            "custom",
            vec![RenderNode::code(None, None, "b")],
            None,
        ),
        RenderNode::inline_code("c"),
    ];
    for dialect in DIALECTS {
        let rendered = render_markdown_node(
            &table_with(POSITIONS[2], cell.clone()),
            &opts(dialect, RenderStrictness::Warn),
        )
        .expect("render");
        let reading = read(&rendered.output);
        let values: Vec<&str> = reading.codes().iter().map(|c| c.value.as_str()).collect();
        assert_eq!(values, ["a", "b", "c"], "{dialect:?}\n{}", rendered.output);
        assert!(!rendered.output.contains("<!--"), "{}", rendered.output);
    }
}
