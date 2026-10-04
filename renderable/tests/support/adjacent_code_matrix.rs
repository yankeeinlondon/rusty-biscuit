//! The neighboring-code fixture matrix: two code values, joined by one
//! boundary shape and wrapped by one inline route, in one of sixteen
//! document contexts.
//!
//! Shared by renderable's writer matrix (`markdown_adjacent_code_spans.rs`)
//! and darkmatter's consumer matrix, which reads the same writer output
//! through darkmatter's public terminal and HTML renderers. Darkmatter
//! includes this file by `#[path]`, so it declares it in
//! `[package.metadata.ci.tests] source-inputs`.

// Each including test binary uses a different subset.
#![allow(dead_code)]

use renderable::style::{Style, TextEmphasis};
use renderable::tree::{
    ColumnAlign, ColumnsHints, HeadingDepth, MarkdownDialect, MarkdownRenderOptions, RenderNode,
    RenderStrictness, SequenceJoin,
};

pub const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];
pub const STRICTNESS: [RenderStrictness; 3] = [
    RenderStrictness::Strict,
    RenderStrictness::Warn,
    RenderStrictness::Lossy,
];

/// Value pairs: equal fences, unequal fences (one and two backticks inside,
/// so two- and three-backtick fences that would combine into a five-run),
/// edge spaces, all-space values, and an empty value on either side.
pub const PAIRS: [(&str, &str, &str); 6] = [
    ("plain", "a", "b"),
    ("unequal fences", "x`y", "z``w"),
    ("edge spaces", " a ", " b "),
    ("all space", "  ", " "),
    ("empty first", "", "b"),
    ("empty second", "a", ""),
];

pub fn opts(dialect: MarkdownDialect, strictness: RenderStrictness) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        dialect,
        strictness,
        style: None,
    }
}

/// A wrapper that a reader sees as a tag around code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tagged {
    Strong,
    Emphasis,
    Delete,
    Link,
}

/// How the two code values meet.
pub struct Boundary {
    pub name: &'static str,
    pub build: fn(RenderNode, RenderNode) -> Vec<RenderNode>,
    /// Visible text between the two values.
    pub between: &'static str,
    /// Visible text after the second value.
    pub after: &'static str,
    /// A tag that wraps the second value only.
    pub second_in: Option<Tagged>,
}

pub fn boundaries() -> Vec<Boundary> {
    let plain = |name, build| Boundary {
        name,
        build,
        between: "",
        after: "",
        second_in: None,
    };
    vec![
        plain("direct", |a, b| vec![a, b]),
        plain("neutral span around second", |a, b| {
            vec![a, RenderNode::span(vec![], vec![b])]
        }),
        plain("empty strong", |a, b| vec![a, RenderNode::strong(vec![]), b]),
        plain("empty text", |a, b| vec![a, RenderNode::text(""), b]),
        plain("empty code", |a, b| vec![a, RenderNode::inline_code(""), b]),
        plain("empty extension", |a, b| {
            vec![a, RenderNode::extended("custom", vec![], None), b]
        }),
        plain("unknown extension around second", |a, b| {
            vec![a, RenderNode::extended("custom", vec![b], None)]
        }),
        plain("class around second", |a, b| {
            vec![a, RenderNode::span(vec!["note".into()], vec![b])]
        }),
        // `⌄` cannot close between two letters, so the dim wrapper is written
        // unstyled and its leading code value meets the first one.
        Boundary {
            name: "unstyled dim around second",
            build: |a, b| {
                vec![
                    a,
                    RenderNode::extended("dim", vec![b, RenderNode::text("q")], None),
                    RenderNode::text("r"),
                ]
            },
            between: "",
            after: "qr",
            second_in: None,
        },
        Boundary {
            between: " ",
            ..plain("visible space", |a, b| vec![a, RenderNode::text(" "), b])
        },
        Boundary {
            between: " ",
            ..plain("soft break", |a, b| vec![a, RenderNode::soft_break(), b])
        },
        Boundary {
            second_in: Some(Tagged::Strong),
            ..plain("strong around second", |a, b| {
                vec![a, RenderNode::strong(vec![b])]
            })
        },
        Boundary {
            second_in: Some(Tagged::Link),
            ..plain("link around second", |a, b| {
                vec![a, RenderNode::link("https://x.test/2", None, vec![b])]
            })
        },
    ]
}

pub type Wrap = fn(Vec<RenderNode>) -> Vec<RenderNode>;

pub fn dim_style() -> Style {
    Style {
        emphasis: TextEmphasis {
            dim: true,
            ..TextEmphasis::default()
        },
        ..Style::default()
    }
}

/// The ten inline routes of the table-cell pipe regressions, plus a dim
/// style (how `Prose` writes `<dim>`), which plain Markdown removes.
pub fn wrappers() -> Vec<(&'static str, Wrap, Option<Tagged>)> {
    vec![
        ("direct", |nodes| nodes, None),
        ("strong", |nodes| vec![RenderNode::strong(nodes)], Some(Tagged::Strong)),
        ("emphasis", |nodes| vec![RenderNode::emphasis(nodes)], Some(Tagged::Emphasis)),
        ("delete", |nodes| vec![RenderNode::delete(nodes)], Some(Tagged::Delete)),
        (
            "link",
            |nodes| vec![RenderNode::link("https://x.test", None, nodes)],
            Some(Tagged::Link),
        ),
        ("neutral span", |nodes| vec![RenderNode::span(vec![], nodes)], None),
        ("classed span", |nodes| vec![RenderNode::span(vec!["note".into()], nodes)], None),
        ("unknown extension", |nodes| vec![RenderNode::extended("custom", nodes, None)], None),
        ("mark", |nodes| vec![RenderNode::extended("mark", nodes, None)], None),
        ("dim", |nodes| vec![RenderNode::extended("dim", nodes, None)], None),
        (
            "dim style",
            |nodes| {
                let mut span = RenderNode::span(vec![], nodes);
                span.attrs.set_style(&dim_style());
                vec![span]
            },
            None,
        ),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub header: bool,
    pub column: usize,
}

pub const POSITIONS: [Position; 4] = [
    Position { header: true, column: 0 },
    Position { header: true, column: 1 },
    Position { header: false, column: 0 },
    Position { header: false, column: 1 },
];

pub const SENTINELS: [[&str; 2]; 2] = [["H1", "H2"], ["y", "z"]];

/// The two-row, two-column fixture of the table-cell regressions, with
/// `cell` at `position`.
pub fn table_with(position: Position, cell: Vec<RenderNode>) -> RenderNode {
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

pub fn depth() -> HeadingDepth {
    HeadingDepth::new(2).expect("valid depth")
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Context {
    Paragraph,
    Composition,
    RootSeparate,
    RootGrouped,
    Heading,
    SectionHeading,
    Cell(Position),
    Quote,
    ListItem,
    Footnote,
    DisclosureSummary,
    DisclosureBody,
    Columns,
}

impl Context {
    /// Whether MarkdownPlus lowers this context to HTML, where each value is
    /// its own `<code>` element.
    pub fn html_in_markdown_plus(self) -> bool {
        matches!(self, Context::DisclosureSummary | Context::Columns)
    }
}

pub fn contexts() -> Vec<Context> {
    let mut all = vec![
        Context::Paragraph,
        Context::Composition,
        Context::RootSeparate,
        Context::RootGrouped,
        Context::Heading,
        Context::SectionHeading,
    ];
    all.extend(POSITIONS.iter().copied().map(Context::Cell));
    all.extend([
        Context::Quote,
        Context::ListItem,
        Context::Footnote,
        Context::DisclosureSummary,
        Context::DisclosureBody,
        Context::Columns,
    ]);
    all
}

pub fn paragraph(text: &str) -> RenderNode {
    RenderNode::paragraph(vec![RenderNode::text(text)])
}

pub fn document(context: Context, target: Vec<RenderNode>) -> RenderNode {
    let after = || paragraph("after");
    match context {
        Context::Paragraph => {
            let mut inline = vec![RenderNode::text("p ")];
            inline.extend(target);
            inline.push(RenderNode::text(" q"));
            RenderNode::root(vec![RenderNode::paragraph(inline), after()])
        }
        Context::Composition => {
            let mut root = RenderNode::root(target);
            root.attrs.set_sequence_join(SequenceJoin::None);
            root
        }
        Context::RootSeparate => RenderNode::root(target),
        Context::RootGrouped => RenderNode::root(vec![RenderNode::span(vec![], target)]),
        Context::Heading | Context::SectionHeading => {
            let mut heading = vec![RenderNode::text("h ")];
            heading.extend(target);
            if context == Context::Heading {
                RenderNode::root(vec![RenderNode::heading(depth(), heading), after()])
            } else {
                RenderNode::root(vec![RenderNode::section(depth(), heading, vec![after()])])
            }
        }
        Context::Cell(position) => table_with(position, target),
        Context::Quote => RenderNode::root(vec![
            RenderNode::block_quote(vec![RenderNode::paragraph(target)]),
            after(),
        ]),
        Context::ListItem => RenderNode::root(vec![RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(None, vec![RenderNode::paragraph(target)])],
        )]),
        Context::Footnote => RenderNode::root(vec![
            RenderNode::paragraph(vec![
                RenderNode::text("r"),
                RenderNode::footnote_reference("n"),
            ]),
            RenderNode::footnote_definition("n", vec![RenderNode::paragraph(target)]),
        ]),
        Context::DisclosureSummary => {
            RenderNode::root(vec![RenderNode::disclosure(target, vec![after()], None)])
        }
        Context::DisclosureBody => RenderNode::root(vec![RenderNode::disclosure(
            vec![RenderNode::text("s")],
            vec![RenderNode::paragraph(target)],
            None,
        )]),
        Context::Columns => {
            let mut columns = RenderNode::block_quote(vec![
                RenderNode::paragraph(target),
                paragraph("right"),
            ]);
            columns.attrs.set_columns_hints(&ColumnsHints {
                left_count: 1,
                ..ColumnsHints::default()
            });
            RenderNode::root(vec![columns])
        }
    }
}

/// One case of the matrix: the two values, how they meet and are wrapped,
/// and the document holding them.
pub struct Case<'a> {
    pub context: Context,
    pub route: &'static str,
    pub route_tag: Option<Tagged>,
    pub boundary: &'a Boundary,
    pub pair: &'static str,
    pub values: (&'static str, &'static str),
    pub node: RenderNode,
}

/// Calls `visit` for every case in `contexts`, skipping the link route
/// around a link boundary (a link cannot hold another link).
pub fn for_each_case(contexts: &[Context], mut visit: impl FnMut(Case<'_>)) {
    let boundaries = boundaries();
    for &context in contexts {
        for (route, wrap, route_tag) in wrappers() {
            for boundary in &boundaries {
                if route_tag == Some(Tagged::Link) && boundary.second_in == Some(Tagged::Link) {
                    continue;
                }
                for (pair, a, b) in PAIRS {
                    let target = wrap((boundary.build)(
                        RenderNode::inline_code(a),
                        RenderNode::inline_code(b),
                    ));
                    visit(Case {
                        context,
                        route,
                        route_tag,
                        boundary,
                        pair,
                        values: (a, b),
                        node: document(context, target),
                    });
                }
            }
        }
    }
}

/// The contexts, grouped one group per matrix test.
pub fn groups() -> [Vec<Context>; 5] {
    [
        vec![
            Context::Paragraph,
            Context::Composition,
            Context::RootSeparate,
            Context::RootGrouped,
        ],
        vec![Context::Heading, Context::SectionHeading],
        POSITIONS.iter().copied().map(Context::Cell).collect(),
        vec![Context::Quote, Context::ListItem, Context::Footnote],
        vec![
            Context::DisclosureSummary,
            Context::DisclosureBody,
            Context::Columns,
        ],
    ]
}
