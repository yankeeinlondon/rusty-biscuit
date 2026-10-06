//! Line endings in quoted fields: generated HTML attribute values and
//! link/image titles.
//!
//! A table row, an ATX heading, and a section heading are one Markdown line,
//! and a blank line ends a paragraph together with any inline HTML tag still
//! open in it. A quoted field keeps its value only if the writer encodes its
//! line endings in a form the field's own reader decodes: a character
//! reference, which both an HTML attribute value and a CommonMark link title
//! decode. A `<br>` would be literal title text, and a literal ending would
//! end the enclosing line.
//!
//! The fields: a generated span `class`; the progress `aria-label`; the four
//! progress glyph `data-*` attributes; a link title; an image title. Each case
//! changes one field of a positive control (`a b`) to hold LF, lone CR, CRLF,
//! or a repeated form, and must then render under the same strictness
//! outcome and diagnostics as its control. Every result is read with an
//! independent GFM reader (`pulldown-cmark`): the table keeps its two rows
//! of two cells with the sentinel cells intact, a heading keeps the target
//! inside it, a paragraph is not split, and the field reads back exactly
//! (an attribute value after HTML decoding, a title as parsed).

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::tree::{
    ColumnAlign, HeadingDepth, MarkdownDialect, MarkdownRenderOptions, ProgressHints, RenderNode,
    RenderStrictness, render_markdown_node,
};

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];
const STRICTNESS: [RenderStrictness; 3] = [
    RenderStrictness::Strict,
    RenderStrictness::Warn,
    RenderStrictness::Lossy,
];

/// The positive control value, then each line-ending shape.
const CONTROL: &str = "a b";
const ENDINGS: [(&str, &str); 6] = [
    ("LF", "a\nb"),
    ("CR", "a\rb"),
    ("CRLF", "a\r\nb"),
    ("LF LF", "a\n\nb"),
    ("CR CR", "a\r\rb"),
    ("CRLF CRLF", "a\r\n\r\nb"),
];

/// A glyph field holds one character: its control and its two endings.
const GLYPH_CONTROL: char = 'x';
const GLYPH_ENDINGS: [char; 2] = ['\n', '\r'];

/// The visible content of every target, distinct from all sentinels.
const TARGET_TEXT: &str = "TGT";

fn opts(dialect: MarkdownDialect, strictness: RenderStrictness) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        dialect,
        strictness,
        style: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
    SpanClass,
    LinkTitle,
    ImageTitle,
    ProgressLabel,
    FillChar,
    EmptyChar,
    LeftBracket,
    RightBracket,
}

impl Field {
    /// The HTML attribute the field is written to, for generated HTML.
    fn attribute(self) -> Option<&'static str> {
        match self {
            Field::SpanClass => Some("class"),
            Field::ProgressLabel => Some("aria-label"),
            Field::FillChar => Some("data-fill-char"),
            Field::EmptyChar => Some("data-empty-char"),
            Field::LeftBracket => Some("data-left-bracket"),
            Field::RightBracket => Some("data-right-bracket"),
            Field::LinkTitle | Field::ImageTitle => None,
        }
    }

    fn is_progress(self) -> bool {
        matches!(
            self,
            Field::ProgressLabel
                | Field::FillChar
                | Field::EmptyChar
                | Field::LeftBracket
                | Field::RightBracket
        )
    }

    /// The control value and each line-ending value with its name.
    fn values(self) -> (String, Vec<(&'static str, String)>) {
        match self {
            Field::FillChar | Field::EmptyChar | Field::LeftBracket | Field::RightBracket => (
                GLYPH_CONTROL.to_string(),
                GLYPH_ENDINGS
                    .iter()
                    .map(|c| (if *c == '\n' { "LF" } else { "CR" }, c.to_string()))
                    .collect(),
            ),
            _ => (
                CONTROL.to_string(),
                ENDINGS.iter().map(|(n, v)| (*n, v.to_string())).collect(),
            ),
        }
    }
}

const PROGRESS_FIELDS: [Field; 5] = [
    Field::ProgressLabel,
    Field::FillChar,
    Field::EmptyChar,
    Field::LeftBracket,
    Field::RightBracket,
];

/// The inline node carrying `value` in an inline field.
fn inline_target(field: Field, value: &str) -> RenderNode {
    match field {
        Field::SpanClass => {
            RenderNode::span(vec![value.to_string()], vec![RenderNode::text(TARGET_TEXT)])
        }
        Field::LinkTitle => RenderNode::link(
            "https://x.test/t",
            Some(value.to_string()),
            vec![RenderNode::text(TARGET_TEXT)],
        ),
        Field::ImageTitle => {
            RenderNode::image("https://x.test/i.png", Some(value.to_string()), TARGET_TEXT)
        }
        _ => unreachable!("not an inline field"),
    }
}

/// A progress paragraph with `value` in a progress field.
fn progress_target(field: Field, value: &str) -> RenderNode {
    let glyph = |default: char| {
        if matches!(field, Field::ProgressLabel) {
            default
        } else {
            value.chars().next().expect("one glyph")
        }
    };
    let defaults = ProgressHints::default();
    let hints = ProgressHints {
        value: 0.6,
        fill_char: if field == Field::FillChar {
            glyph(defaults.fill_char)
        } else {
            defaults.fill_char
        },
        empty_char: if field == Field::EmptyChar {
            glyph(defaults.empty_char)
        } else {
            defaults.empty_char
        },
        left_bracket: if field == Field::LeftBracket {
            glyph(defaults.left_bracket)
        } else {
            defaults.left_bracket
        },
        right_bracket: if field == Field::RightBracket {
            glyph(defaults.right_bracket)
        } else {
            defaults.right_bracket
        },
        ..defaults
    };
    let label = if field == Field::ProgressLabel {
        value
    } else {
        "Load"
    };
    let mut paragraph = RenderNode::paragraph(vec![RenderNode::text(format!("{label} 60%"))]);
    paragraph.attrs.set_progress_hints(&hints);
    paragraph
}

type Wrap = fn(Vec<RenderNode>) -> Vec<RenderNode>;

/// The inline routes a field takes into its container (the same ten the
/// table-cell pipe regressions use).
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

const SENTINELS: [[&str; 2]; 2] = [["H1", "H2"], ["y", "z"]];

/// The two-row, two-column fixture of the table-cell pipe regressions, with
/// `cell` at `position`.
fn table_with(position: Position, cell: Vec<RenderNode>) -> RenderNode {
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
    RenderNode::root(vec![RenderNode::table(
        vec![ColumnAlign::None, ColumnAlign::None],
        rows,
    )])
}

fn depth() -> HeadingDepth {
    HeadingDepth::new(2).expect("valid depth")
}

/// Where the target sits.
#[derive(Debug, Clone, Copy)]
enum Context {
    Cell(Position),
    Heading,
    SectionHeading,
    Paragraph,
}

fn contexts() -> Vec<Context> {
    let mut all: Vec<Context> = POSITIONS.iter().copied().map(Context::Cell).collect();
    all.extend([Context::Heading, Context::SectionHeading, Context::Paragraph]);
    all
}

/// The document holding `target` (inline nodes, or one progress paragraph)
/// in `context`. A progress paragraph reaches a heading through an inline
/// extension, which writes it on the heading's line.
fn document(context: Context, target: Vec<RenderNode>, progress: bool) -> RenderNode {
    let in_heading = |target: Vec<RenderNode>| {
        let mut heading = vec![RenderNode::text("h ")];
        if progress {
            heading.push(RenderNode::extended("custom", target, None));
        } else {
            heading.extend(target);
        }
        heading
    };
    match context {
        Context::Cell(position) => table_with(position, target),
        Context::Heading => RenderNode::root(vec![
            RenderNode::heading(depth(), in_heading(target)),
            RenderNode::paragraph(vec![RenderNode::text("after")]),
        ]),
        Context::SectionHeading => RenderNode::root(vec![RenderNode::section(
            depth(),
            in_heading(target),
            vec![RenderNode::paragraph(vec![RenderNode::text("after")])],
        )]),
        Context::Paragraph => {
            let first = if progress {
                target.into_iter().next().expect("progress paragraph")
            } else {
                let mut inline = vec![RenderNode::text("p ")];
                inline.extend(target);
                inline.push(RenderNode::text(" q"));
                RenderNode::paragraph(inline)
            };
            RenderNode::root(vec![
                first,
                RenderNode::paragraph(vec![RenderNode::text("after")]),
            ])
        }
    }
}

fn parse(markdown: &str) -> Vec<Event<'_>> {
    Parser::new_ext(
        markdown,
        Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_STRIKETHROUGH,
    )
    .collect()
}

/// What a top-level block or table cell read as: its visible text, its
/// inline HTML, and the titles of its links and images.
#[derive(Debug, Default)]
struct Reading {
    text: String,
    html: String,
    link_titles: Vec<String>,
    image_titles: Vec<String>,
}

impl Reading {
    fn push(&mut self, event: &Event<'_>) {
        match event {
            Event::Text(text) | Event::Code(text) => self.text.push_str(text),
            Event::InlineHtml(html) | Event::Html(html) => self.html.push_str(html),
            Event::Start(Tag::Link { title, .. }) => self.link_titles.push(title.to_string()),
            Event::Start(Tag::Image { title, .. }) => self.image_titles.push(title.to_string()),
            _ => {}
        }
    }
}

/// The document's shape: table cells row by row (if one table), and the
/// top-level blocks with their kind.
#[derive(Debug, Default)]
struct Shape {
    tables: usize,
    rows: Vec<Vec<Reading>>,
    blocks: Vec<(String, Reading)>,
}

fn read(markdown: &str) -> Shape {
    let mut shape = Shape::default();
    let mut depth = 0usize;
    let mut cell: Option<Reading> = None;
    for event in parse(markdown) {
        match &event {
            Event::Start(tag) => {
                if depth == 0 {
                    let kind = match tag {
                        Tag::Heading { .. } => "heading",
                        Tag::Paragraph => "paragraph",
                        Tag::Table(_) => "table",
                        Tag::HtmlBlock => "html",
                        _ => "other",
                    };
                    if kind == "table" {
                        shape.tables += 1;
                    }
                    shape.blocks.push((kind.to_string(), Reading::default()));
                }
                depth += 1;
                match tag {
                    Tag::TableHead | Tag::TableRow => shape.rows.push(Vec::new()),
                    Tag::TableCell => cell = Some(Reading::default()),
                    _ => {}
                }
            }
            Event::End(tag) => {
                depth -= 1;
                if *tag == TagEnd::TableCell {
                    let value = cell.take().expect("cell open");
                    shape.rows.last_mut().expect("row open").push(value);
                }
            }
            _ => {}
        }
        if let Some(reading) = cell.as_mut() {
            reading.push(&event);
        }
        if let Some((_, reading)) = shape.blocks.last_mut() {
            reading.push(&event);
        }
    }
    shape
}

/// Decodes the character and entity references HTML generation can write.
fn decode_html(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest.find(';').expect("terminated reference");
        let name = &rest[1..end];
        let decoded = match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ if name.starts_with("#x") || name.starts_with("#X") => {
                char::from_u32(u32::from_str_radix(&name[2..], 16).expect("hex")).expect("char")
            }
            _ if name.starts_with('#') => {
                char::from_u32(name[1..].parse().expect("decimal")).expect("char")
            }
            other => panic!("unexpected reference &{other};"),
        };
        out.push(decoded);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Every decoded value of the double-quoted `name` attribute in `html`.
fn attribute_values(html: &str, name: &str) -> Vec<String> {
    let needle = format!(" {name}=\"");
    let mut values = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find(&needle) {
        rest = &rest[at + needle.len()..];
        let end = rest.find('"').expect("closing quote");
        values.push(decode_html(&rest[..end]));
        rest = &rest[end + 1..];
    }
    values
}

/// Problems with how `reading` carries the target: its visible content and,
/// where the dialect writes the field, its exact value.
fn check_target(
    reading: &Reading,
    field: Field,
    value: &str,
    dialect: MarkdownDialect,
    problems: &mut Vec<String>,
) {
    let expected = value.to_string();
    let visible = if field.is_progress() {
        // The progress paragraph's percentage is visible in both dialects.
        "60%"
    } else {
        TARGET_TEXT
    };
    if !reading.text.contains(visible) {
        problems.push(format!("visible {visible:?} missing from {reading:?}"));
    }
    match field {
        Field::LinkTitle => {
            let titles: Vec<&String> = reading.link_titles.iter().filter(|t| !t.is_empty()).collect();
            if titles != [&expected] {
                problems.push(format!("link titles {titles:?}, expected [{expected:?}]"));
            }
        }
        Field::ImageTitle => {
            if reading.image_titles != [expected.clone()] {
                problems.push(format!(
                    "image titles {:?}, expected [{expected:?}]",
                    reading.image_titles
                ));
            }
        }
        _ if dialect == MarkdownDialect::MarkdownPlus => {
            let attribute = field.attribute().expect("attribute field");
            let values = attribute_values(&reading.html, attribute);
            if !values.contains(&expected) {
                problems.push(format!(
                    "{attribute} values {values:?} lack {expected:?} in {:?}",
                    reading.html
                ));
            }
        }
        // Plain Markdown writes no class or progress attribute: the class is
        // removed and progress falls back to its text.
        _ => {}
    }
}

/// Problems with the structure of `markdown` for `context`, plus the reading
/// of the target.
fn check_document(
    markdown: &str,
    context: Context,
    field: Field,
    value: &str,
    dialect: MarkdownDialect,
) -> Vec<String> {
    let shape = read(markdown);
    let mut problems = Vec::new();
    match context {
        Context::Cell(position) => {
            if shape.tables != 1 || shape.blocks.len() != 1 {
                problems.push(format!("blocks {:?}", shape.blocks));
                return problems;
            }
            if shape.rows.len() != 2 || shape.rows.iter().any(|row| row.len() != 2) {
                problems.push(format!("rows {:?}", shape.rows));
                return problems;
            }
            for (row, cells) in shape.rows.iter().enumerate() {
                for (column, cell) in cells.iter().enumerate() {
                    let target = (row == 0) == position.header && column == position.column;
                    if target {
                        check_target(cell, field, value, dialect, &mut problems);
                    } else if cell.text != SENTINELS[row][column] || !cell.html.is_empty() {
                        problems.push(format!("sentinel {row},{column} read {cell:?}"));
                    }
                }
            }
        }
        Context::Heading | Context::SectionHeading | Context::Paragraph => {
            let kinds: Vec<&str> = shape.blocks.iter().map(|(k, _)| k.as_str()).collect();
            let first = if matches!(context, Context::Paragraph) {
                "paragraph"
            } else {
                "heading"
            };
            if kinds != [first, "paragraph"] || shape.blocks[1].1.text != "after" {
                problems.push(format!("blocks {:?}", shape.blocks));
                return problems;
            }
            check_target(&shape.blocks[0].1, field, value, dialect, &mut problems);
        }
    }
    problems
}

/// Renders `node`, returning the output and diagnostic count, or `None` when
/// the strictness rejects it.
fn render(node: &RenderNode, options: &MarkdownRenderOptions) -> Option<(String, usize)> {
    render_markdown_node(node, options)
        .ok()
        .map(|rendered| (rendered.output, rendered.diagnostics.len()))
}

/// Runs one field through every context, wrapper, dialect, strictness, and
/// value; returns the failures and the number of cases checked.
fn run_field(field: Field) -> (Vec<String>, usize) {
    let (control, endings) = field.values();
    let routes: Vec<(&str, Wrap)> = if field.is_progress() {
        vec![("direct", |nodes| nodes)]
    } else {
        wrappers()
    };
    let target = |value: &str| {
        if field.is_progress() {
            vec![progress_target(field, value)]
        } else {
            vec![inline_target(field, value)]
        }
    };
    let mut failures = Vec::new();
    let mut cases = 0;
    for context in contexts() {
        for (route, wrap) in &routes {
            for dialect in DIALECTS {
                if field == Field::ProgressLabel
                    && dialect == MarkdownDialect::Markdown
                    && matches!(context, Context::Paragraph)
                {
                    // Plain Markdown writes the progress paragraph as its
                    // text, whose blank line separates paragraphs by the
                    // text policy; no quoted field is involved.
                    continue;
                }
                for strictness in STRICTNESS {
                    let options = opts(dialect, strictness);
                    let control_doc = document(context, wrap(target(&control)), field.is_progress());
                    let control_result = render(&control_doc, &options);
                    let label = format!("{field:?} {context:?} {route} {dialect:?} {strictness:?}");
                    if let Some((markdown, _)) = &control_result {
                        let problems = check_document(markdown, context, field, &control, dialect);
                        if !problems.is_empty() {
                            failures.push(format!("{label} control: {problems:?}\n{markdown}"));
                        }
                    }
                    for (ending, value) in &endings {
                        cases += 1;
                        let doc = document(context, wrap(target(value)), field.is_progress());
                        let result = render(&doc, &options);
                        let label = format!("{label} {ending}");
                        match (&control_result, &result) {
                            (None, None) => {}
                            (Some((_, control_diags)), Some((markdown, diags))) => {
                                if diags != control_diags {
                                    failures.push(format!(
                                        "{label}: {diags} diagnostics, control {control_diags}"
                                    ));
                                }
                                let problems = check_document(markdown, context, field, value, dialect);
                                if !problems.is_empty() {
                                    failures.push(format!("{label}: {problems:?}\n{markdown}"));
                                }
                            }
                            (control, result) => failures.push(format!(
                                "{label}: control accepted {}, case accepted {}",
                                control.is_some(),
                                result.is_some()
                            )),
                        }
                    }
                }
            }
        }
    }
    (failures, cases)
}

fn assert_field_matrix(fields: &[Field]) {
    let mut failures = Vec::new();
    let mut cases = 0;
    for field in fields {
        let (field_failures, field_cases) = run_field(*field);
        failures.extend(field_failures);
        cases += field_cases;
    }
    assert!(cases > 0);
    assert!(
        failures.is_empty(),
        "{} of {cases} cases failed; first ten:\n{}",
        failures.len(),
        failures.iter().take(10).cloned().collect::<Vec<_>>().join("\n\n")
    );
}

#[test]
fn span_class_line_endings_keep_the_line_and_the_attribute_value() {
    assert_field_matrix(&[Field::SpanClass]);
}

#[test]
fn link_and_image_title_line_endings_read_back_exactly() {
    assert_field_matrix(&[Field::LinkTitle, Field::ImageTitle]);
}

#[test]
fn progress_label_and_glyph_line_endings_keep_the_line_and_attribute_values() {
    assert_field_matrix(&PROGRESS_FIELDS);
}

/// Fields with their own line-ending policies, which the quoted-field
/// encoding must not touch: text, a link label, inline code, a link
/// destination, and image alt text. Each keeps the table and heading intact,
/// and in a heading text, a link label, and alt text read back exactly (a
/// line ending there is a character reference, alt text included).
#[test]
fn other_fields_keep_the_line_under_their_own_policies() {
    type Build = fn(&str) -> RenderNode;
    let fields: [(&str, Build); 5] = [
        ("text", |v| RenderNode::text(v)),
        ("link label", |v| {
            RenderNode::link("https://x.test", None, vec![RenderNode::text(v)])
        }),
        ("inline code", |v| RenderNode::inline_code(v)),
        ("destination", |v| {
            RenderNode::link(format!("https://x.test/{v}"), None, vec![RenderNode::text("d")])
        }),
        ("image alt", |v| RenderNode::image("https://x.test/i.png", None, v)),
    ];
    let mut failures = Vec::new();
    for (name, build) in fields {
        for (ending, value) in ENDINGS {
            for dialect in DIALECTS {
                let options = opts(dialect, RenderStrictness::Strict);
                for context in [
                    Context::Cell(POSITIONS[0]),
                    Context::Cell(POSITIONS[3]),
                    Context::Heading,
                ] {
                    let doc = document(context, vec![build(value)], false);
                    let markdown = render_markdown_node(&doc, &options)
                        .expect("renders under Strict")
                        .output;
                    let shape = read(&markdown);
                    let intact = match context {
                        Context::Cell(_) => {
                            shape.tables == 1
                                && shape.rows.len() == 2
                                && shape.rows.iter().all(|row| row.len() == 2)
                        }
                        _ => {
                            shape.blocks.len() == 2
                                && shape.blocks[0].0 == "heading"
                                && shape.blocks[1].1.text == "after"
                        }
                    };
                    let exact_in_heading = matches!(name, "text" | "link label" | "image alt");
                    if !intact {
                        failures.push(format!("{name} {ending} {dialect:?} {context:?}:\n{markdown}"));
                    } else if matches!(context, Context::Heading)
                        && exact_in_heading
                        && shape.blocks[0].1.text != format!("h {value}")
                    {
                        failures.push(format!(
                            "{name} {ending} {dialect:?} heading read {:?}:\n{markdown}",
                            shape.blocks[0].1.text
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
