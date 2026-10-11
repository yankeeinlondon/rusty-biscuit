//! Footnote identifiers a Markdown label cannot spell.
//!
//! A reader matches a footnote label without processing its escapes, so an
//! identifier holding an unescaped `[` or `]`, ending in an odd backslash
//! run, holding a line ending or a whitespace run, or holding no
//! non-whitespace character cannot be written so that it reads back as
//! itself. The writer applies the strictness model to the reference and the
//! definition alike: `Strict` rejects the tree; `Warn` and `Lossy` write the
//! same degraded label in both places, so they still pair, and `Warn`
//! records one lossy diagnostic for each.
//!
//! Each case is read back with an independent GFM reader (`pulldown-cmark`).

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use renderable::tree::{
    ColumnAlign, DiagnosticKind, HeadingDepth, MarkdownDialect, MarkdownRenderOptions,
    RenderError, RenderNode, RenderStrictness, render_markdown_node,
};

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

fn opts(dialect: MarkdownDialect, strictness: RenderStrictness) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        dialect,
        strictness,
        style: None,
    }
}

/// Unrepresentable identifiers and the label `Warn`/`Lossy` write for each.
const DEGRADED: [(&str, &str); 11] = [
    ("a]b", "a\\]b"),
    ("a[b", "a\\[b"),
    ("a\\\\]b", "a\\\\\\]b"),
    ("a\\", "a\\\\"),
    ("", "_"),
    ("  ", "_"),
    ("\n", "_"),
    ("a\nb", "a b"),
    ("a\r\nb", "a b"),
    (" a \t b ", "a b"),
    ("x]\n", "x\\]"),
];

/// Identifiers that already read back as themselves.
const CLEAN: [&str; 6] = ["n", "a\\]b", "a\\b", "a\\\\", "a|b", "^x y"];

/// Where the reference sits.
#[derive(Debug, Clone, Copy)]
enum Place {
    Paragraph,
    Heading,
    /// A table cell: header or body row, first or second column.
    Cell(bool, usize),
}

const PLACES: [Place; 6] = [
    Place::Paragraph,
    Place::Heading,
    Place::Cell(true, 0),
    Place::Cell(true, 1),
    Place::Cell(false, 0),
    Place::Cell(false, 1),
];

const SENTINELS: [[&str; 2]; 2] = [["H1", "H2"], ["y", "z"]];

/// A document with a reference to `identifier` at `place`, followed by its
/// definition.
fn document(place: Place, identifier: &str) -> RenderNode {
    let reference = RenderNode::footnote_reference(identifier);
    let site = match place {
        Place::Paragraph => RenderNode::paragraph(vec![
            RenderNode::text("p"),
            reference,
            RenderNode::text("q"),
        ]),
        Place::Heading => RenderNode::heading(
            HeadingDepth::new(2).expect("valid depth"),
            vec![RenderNode::text("h"), reference],
        ),
        Place::Cell(header, column) => {
            let mut reference = Some(reference);
            RenderNode::table(
                vec![ColumnAlign::None, ColumnAlign::None],
                (0..2)
                    .map(|row| {
                        RenderNode::table_row(
                            (0..2)
                                .map(|col| {
                                    RenderNode::table_cell(
                                        if (row == 0) == header && col == column {
                                            vec![reference.take().expect("one reference")]
                                        } else {
                                            vec![RenderNode::text(SENTINELS[row][col])]
                                        },
                                    )
                                })
                                .collect(),
                        )
                    })
                    .collect(),
            )
        }
    };
    RenderNode::root(vec![
        site,
        RenderNode::footnote_definition(identifier, vec![RenderNode::text("note")]),
    ])
}

/// What a reader finds: every footnote reference label, every definition
/// label, and, for a table, its cells (a reference reads as `[^label]`).
struct Reading {
    references: Vec<String>,
    definitions: Vec<String>,
    cells: Vec<Vec<String>>,
}

fn read(markdown: &str) -> Reading {
    let mut reading = Reading {
        references: Vec::new(),
        definitions: Vec::new(),
        cells: Vec::new(),
    };
    let mut cell: Option<String> = None;
    let parser = Parser::new_ext(markdown, Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES);
    for event in parser {
        match &event {
            Event::Start(Tag::TableHead | Tag::TableRow) => reading.cells.push(Vec::new()),
            Event::Start(Tag::TableCell) => cell = Some(String::new()),
            Event::End(TagEnd::TableCell) => {
                let value = cell.take().expect("cell open");
                reading.cells.last_mut().expect("row open").push(value);
            }
            Event::Start(Tag::FootnoteDefinition(label)) => {
                reading.definitions.push(label.to_string());
            }
            Event::FootnoteReference(label) => {
                reading.references.push(label.to_string());
                if let Some(value) = cell.as_mut() {
                    value.push_str(&format!("[^{label}]"));
                }
            }
            Event::Text(text) => {
                if let Some(value) = cell.as_mut() {
                    value.push_str(text);
                }
            }
            _ => {}
        }
    }
    reading
}

/// Checks the reading: one reference and one definition, both `label`, and
/// for a table every sentinel intact around the reference's cell.
fn check(place: Place, markdown: &str, label: &str) -> Result<(), String> {
    let reading = read(markdown);
    if reading.references != [label] || reading.definitions != [label] {
        return Err(format!(
            "references {:?}, definitions {:?}, wanted {label:?}",
            reading.references, reading.definitions
        ));
    }
    if let Place::Cell(header, column) = place {
        let mut wanted: Vec<Vec<String>> = SENTINELS
            .iter()
            .map(|row| row.iter().map(|v| (*v).to_string()).collect())
            .collect();
        wanted[usize::from(!header)][column] = format!("[^{label}]");
        if reading.cells != wanted {
            return Err(format!("cells {:?}, wanted {wanted:?}", reading.cells));
        }
    }
    Ok(())
}

#[test]
fn an_unrepresentable_identifier_follows_the_strictness_model_in_every_place() {
    let mut failures = Vec::new();
    let mut cases = 0;
    for (identifier, label) in DEGRADED {
        for dialect in DIALECTS {
            for place in PLACES {
                cases += 1;
                let case = format!("{identifier:?} / {dialect:?} / {place:?}");
                let tree = document(place, identifier);

                match render_markdown_node(&tree, &opts(dialect, RenderStrictness::Strict)) {
                    Err(RenderError::LossyRejected { message })
                        if message.contains("footnote identifier") => {}
                    other => failures.push(format!("{case}: Strict gave {other:?}")),
                }

                let warn = render_markdown_node(&tree, &opts(dialect, RenderStrictness::Warn))
                    .expect("Warn renders");
                let lossy = render_markdown_node(&tree, &opts(dialect, RenderStrictness::Lossy))
                    .expect("Lossy renders");
                // One for the reference, one for the definition.
                if warn.diagnostics.len() != 2
                    || warn.diagnostics.iter().any(|d| {
                        d.kind != DiagnosticKind::Lossy || !d.message.contains("footnote identifier")
                    })
                {
                    failures.push(format!("{case}: Warn diagnostics {:?}", warn.diagnostics));
                }
                if !lossy.diagnostics.is_empty() || lossy.output != warn.output {
                    failures.push(format!(
                        "{case}: Lossy {:?} differs from Warn:\n{}\n{}",
                        lossy.diagnostics, warn.output, lossy.output
                    ));
                }
                if let Err(failure) = check(place, &warn.output, label) {
                    failures.push(format!("{case}: {failure}, from:\n{}", warn.output));
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
fn a_representable_identifier_is_written_as_is_in_every_place() {
    for identifier in CLEAN {
        for dialect in DIALECTS {
            for place in PLACES {
                let rendered = render_markdown_node(
                    &document(place, identifier),
                    &opts(dialect, RenderStrictness::Strict),
                )
                .unwrap_or_else(|error| panic!("{identifier:?} {place:?}: {error}"));
                assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
                assert_eq!(
                    check(place, &rendered.output, identifier),
                    Ok(()),
                    "{identifier:?} {dialect:?} {place:?}\n{}",
                    rendered.output
                );
            }
        }
    }
}

#[test]
fn a_definition_without_a_reference_degrades_its_label_alone() {
    let tree = RenderNode::root(vec![RenderNode::footnote_definition(
        "a]b",
        vec![RenderNode::text("note")],
    )]);
    let rendered = render_markdown_node(
        &tree,
        &opts(MarkdownDialect::Markdown, RenderStrictness::Warn),
    )
    .expect("Warn renders");
    assert_eq!(rendered.output, "[^a\\]b]: note");
    assert_eq!(rendered.diagnostics.len(), 1, "{:?}", rendered.diagnostics);
}
