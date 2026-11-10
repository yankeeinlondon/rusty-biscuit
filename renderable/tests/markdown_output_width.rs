//! The Markdown fold's output width.
//!
//! `Width::Source` keeps soft breaks as authored, `Width::Unlimited` joins
//! them, and `Width::Columns(n)` joins and wraps each paragraph at `n`
//! display columns. A wrapped paragraph must read back exactly as the joined
//! one: each case is read with an independent GFM reader (`pulldown-cmark`)
//! and compared with the `Unlimited` rendering, a soft break counting as the
//! space it stands for.

use pulldown_cmark::{Event, Options, Parser};
use renderable::output::Width;
use renderable::tree::{
    ColumnAlign, HeadingDepth, MarkdownDialect, MarkdownRenderOptions, RenderNode,
    RenderStrictness, render_markdown_node,
};
use unicode_width::UnicodeWidthStr;

const DIALECTS: [MarkdownDialect; 2] = [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus];

fn opts(dialect: MarkdownDialect, width: Width) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        dialect,
        strictness: RenderStrictness::Warn,
        style: None,
        width,
    }
}

fn render(node: &RenderNode, dialect: MarkdownDialect, width: Width) -> String {
    render_markdown_node(node, &opts(dialect, width))
        .expect("the tree renders")
        .output
}

/// The reader's view of `markdown`: structure and text, with soft breaks
/// read as spaces and adjacent text merged.
fn read(markdown: &str) -> Vec<String> {
    let mut events = Vec::new();
    let mut text = String::new();
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_STRIKETHROUGH;
    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Text(value) => text.push_str(&value),
            Event::SoftBreak => text.push(' '),
            other => {
                if !text.is_empty() {
                    events.push(format!("Text({text:?})"));
                    text.clear();
                }
                events.push(format!("{other:?}"));
            }
        }
    }
    if !text.is_empty() {
        events.push(format!("Text({text:?})"));
    }
    events
}

/// Asserts that every wrapped width reads back as the joined rendering.
fn assert_wraps_faithfully(node: &RenderNode, widths: impl IntoIterator<Item = u32>) {
    let widths: Vec<u32> = widths.into_iter().collect();
    for dialect in DIALECTS {
        let joined = render(node, dialect, Width::Unlimited);
        let expected = read(&joined);
        for &columns in &widths {
            let wrapped = render(node, dialect, Width::Columns(columns));
            assert_eq!(
                read(&wrapped),
                expected,
                "{dialect:?} at {columns} columns changed meaning:\n{wrapped}\n--- joined:\n{joined}"
            );
        }
    }
}

fn paragraph(text: &str) -> RenderNode {
    RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text(text)])])
}

fn widest_line(markdown: &str) -> usize {
    markdown
        .lines()
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0)
}

const PROSE: &str = "The quick brown fox jumps over the lazy dog while the cat watches \
                     from the warm windowsill and wonders why anyone would run at all";

#[test]
fn source_width_keeps_soft_breaks_byte_for_byte() {
    let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
        RenderNode::text("one"),
        RenderNode::soft_break(),
        RenderNode::text("two"),
    ])]);
    for dialect in DIALECTS {
        let default = render_markdown_node(
            &tree,
            &MarkdownRenderOptions {
                dialect,
                ..MarkdownRenderOptions::default()
            },
        )
        .expect("the tree renders")
        .output;
        assert_eq!(default, "one\ntwo");
        assert_eq!(render(&tree, dialect, Width::Source), default);
    }
}

#[test]
fn unlimited_joins_soft_breaks_and_keeps_hard_breaks() {
    let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
        RenderNode::text("one"),
        RenderNode::soft_break(),
        RenderNode::text("two"),
        RenderNode::hard_break(),
        RenderNode::text("three"),
        RenderNode::soft_break(),
        RenderNode::text(PROSE),
    ])]);
    for dialect in DIALECTS {
        let out = render(&tree, dialect, Width::Unlimited);
        assert_eq!(out, format!("one two\\\nthree {PROSE}"));
    }
}

#[test]
fn columns_wraps_paragraphs_at_word_boundaries() {
    let tree = paragraph(PROSE);
    for dialect in DIALECTS {
        let out = render(&tree, dialect, Width::Columns(20));
        assert!(out.lines().count() > 1, "{out}");
        assert!(widest_line(&out) <= 20, "{out}");
        assert_eq!(out.replace('\n', " "), PROSE);
    }
    assert_wraps_faithfully(&tree, [8, 20, 33, 80]);
}

#[test]
fn columns_joins_soft_breaks_before_wrapping_and_restarts_after_hard_breaks() {
    let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
        RenderNode::text("aaa bbb"),
        RenderNode::soft_break(),
        RenderNode::text("ccc"),
        RenderNode::hard_break(),
        RenderNode::text("dddd eeee ffff"),
    ])]);
    // The hard break's backslash is on the line, so it counts.
    let out = render(&tree, MarkdownDialect::Markdown, Width::Columns(12));
    assert_eq!(out, "aaa bbb ccc\\\ndddd eeee\nffff");
    let out = render(&tree, MarkdownDialect::Markdown, Width::Columns(11));
    assert_eq!(out, "aaa bbb\nccc\\\ndddd eeee\nffff");
    assert_wraps_faithfully(&tree, [3, 11, 40]);
}

#[test]
fn code_spans_stay_on_one_line() {
    let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
        RenderNode::text("run "),
        RenderNode::inline_code("cargo nextest run --workspace --color never"),
        RenderNode::text(" before you push the branch"),
    ])]);
    for dialect in DIALECTS {
        let out = render(&tree, dialect, Width::Columns(12));
        assert!(
            out.lines()
                .any(|line| line.contains("`cargo nextest run --workspace --color never`")),
            "{out}"
        );
    }
    assert_wraps_faithfully(&tree, [4, 12, 30]);
}

#[test]
fn link_destination_and_title_stay_with_the_closing_bracket() {
    let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
        RenderNode::text("see "),
        RenderNode::link(
            "https://example.com/a/long/path",
            Some("a title with spaces".into()),
            vec![RenderNode::text("the docs page")],
        ),
        RenderNode::text(" for more detail on this"),
    ])]);
    for dialect in DIALECTS {
        let out = render(&tree, dialect, Width::Columns(10));
        let tail = r#"](https://example.com/a/long/path "a title with spaces")"#;
        let line = out
            .lines()
            .find(|line| line.contains("](https"))
            .unwrap_or_else(|| panic!("no link tail in {out}"));
        assert!(line.contains(&format!("page{tail}")), "{out}");
        assert!(!out.lines().any(|line| line.starts_with(']')), "{out}");
    }
    assert_wraps_faithfully(&tree, [1, 10, 25, 60]);
}

#[test]
fn a_url_link_and_a_long_word_overflow_rather_than_split() {
    let url = "https://example.com/a/very/long/path/that/does/not/fit";
    let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
        RenderNode::text("go to "),
        RenderNode::link(url, None, vec![RenderNode::text(url)]),
        RenderNode::text(" or Pneumonoultramicroscopicsilicovolcanoconiosis now"),
    ])]);
    for dialect in DIALECTS {
        let out = render(&tree, dialect, Width::Columns(16));
        assert!(
            out.lines().any(|line| line == format!("[{url}]({url})")),
            "{out}"
        );
        assert!(
            out.lines()
                .any(|line| line == "Pneumonoultramicroscopicsilicovolcanoconiosis"),
            "{out}"
        );
    }
    assert_wraps_faithfully(&tree, [5, 16]);
}

#[test]
fn block_quote_and_list_prefixes_count_toward_the_width() {
    let tree = RenderNode::root(vec![
        RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text(PROSE)])]),
        RenderNode::list(
            true,
            Some(9),
            vec![
                RenderNode::list_item(
                    None,
                    vec![RenderNode::paragraph(vec![RenderNode::text(PROSE)])],
                ),
                RenderNode::list_item(
                    Some(true),
                    vec![
                        RenderNode::paragraph(vec![RenderNode::text(PROSE)]),
                        RenderNode::list(
                            false,
                            None,
                            vec![RenderNode::list_item(
                                None,
                                vec![RenderNode::paragraph(vec![RenderNode::text(PROSE)])],
                            )],
                        ),
                    ],
                ),
            ],
        ),
        RenderNode::block_quote(vec![RenderNode::block_quote(vec![RenderNode::paragraph(
            vec![RenderNode::text(PROSE)],
        )])]),
    ]);
    for dialect in DIALECTS {
        let out = render(&tree, dialect, Width::Columns(24));
        assert!(widest_line(&out) <= 24, "{out}");
        assert!(out.lines().any(|line| line.starts_with("> > ")), "{out}");
        assert!(
            out.lines().any(|line| line.starts_with("10. [x] ")),
            "{out}"
        );
    }
    assert_wraps_faithfully(&tree, [12, 24, 50]);
}

#[test]
fn code_blocks_tables_and_headings_are_never_reflowed() {
    let tree = RenderNode::root(vec![
        RenderNode::heading(
            HeadingDepth::new(2).expect("valid depth"),
            vec![RenderNode::text(
                "A heading far wider than the requested width",
            )],
        ),
        RenderNode::code(
            Some("rust".into()),
            None,
            "let value = some_function(with, many, arguments, that, are, long);\n",
        ),
        RenderNode::table(
            vec![ColumnAlign::None, ColumnAlign::None],
            vec![
                RenderNode::table_row(vec![
                    RenderNode::table_cell(vec![RenderNode::text("a wide header cell")]),
                    RenderNode::table_cell(vec![RenderNode::text("another wide header")]),
                ]),
                RenderNode::table_row(vec![
                    RenderNode::table_cell(vec![
                        RenderNode::text("body"),
                        RenderNode::soft_break(),
                        RenderNode::text("cell text"),
                    ]),
                    RenderNode::table_cell(vec![RenderNode::text("more body text")]),
                ]),
            ],
        ),
    ]);
    for dialect in DIALECTS {
        let source = render(&tree, dialect, Width::Source);
        assert_eq!(render(&tree, dialect, Width::Unlimited), source);
        assert_eq!(render(&tree, dialect, Width::Columns(10)), source);
    }
}

#[test]
fn a_wrap_never_starts_a_block() {
    let texts = [
        "word # not a heading and ## nor this ###### either",
        "word - not a list + nor this * nor this",
        "word 1. not ordered 2) nor this 10. nor this",
        "word > not a quote >> nor this",
        "word --- not a rule === nor underline ___ nor this *** nor this",
        "word ``` not a fence ~~~ nor this",
        "word | --- | :-: | not a delimiter row",
        "word <div> not html <!-- nor a comment",
        "word :: not a directive ::disclosure nor this",
        "word = still text - and more - text",
    ];
    for text in texts {
        let tree = RenderNode::root(vec![
            RenderNode::paragraph(vec![RenderNode::text(text)]),
            RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text(text)])]),
            RenderNode::list(
                false,
                None,
                vec![RenderNode::list_item(
                    None,
                    vec![RenderNode::paragraph(vec![RenderNode::text(text)])],
                )],
            ),
        ]);
        assert_wraps_faithfully(&tree, 1..=40);
    }
}

#[test]
fn a_wrap_never_turns_a_backslash_into_a_hard_break() {
    let tree = paragraph("one\\ two\\\\ three \\ four");
    assert_wraps_faithfully(&tree, 1..=20);
}

#[test]
fn emphasis_strong_and_spans_wrap_inside_and_keep_their_meaning() {
    let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
        RenderNode::text("plain words then "),
        RenderNode::emphasis(vec![RenderNode::text(
            "an emphasized phrase that runs long",
        )]),
        RenderNode::text(" and "),
        RenderNode::strong(vec![RenderNode::text("a strong one as well")]),
        RenderNode::text(" then "),
        RenderNode::span(
            vec!["note".into()],
            vec![RenderNode::text("a classed span body with words")],
        ),
        RenderNode::text(" done"),
    ])]);
    let out = render(&tree, MarkdownDialect::Markdown, Width::Columns(14));
    assert!(widest_line(&out) <= 14, "{out}");
    assert_wraps_faithfully(&tree, 1..=40);
}

#[test]
fn wide_characters_count_by_display_width() {
    let tree = paragraph("日本語 の 文章 を 折り返す テスト です");
    let out = render(&tree, MarkdownDialect::Markdown, Width::Columns(10));
    assert!(widest_line(&out) <= 10, "{out}");
    assert_wraps_faithfully(&tree, [4, 10]);
}

#[test]
fn markdown_plus_wraps_like_markdown() {
    let tree = RenderNode::root(vec![
        RenderNode::paragraph(vec![RenderNode::text(PROSE)]),
        RenderNode::block_quote(vec![RenderNode::paragraph(vec![
            RenderNode::text("a "),
            RenderNode::inline_code("code span"),
            RenderNode::soft_break(),
            RenderNode::text(PROSE),
        ])]),
    ]);
    for width in [Width::Unlimited, Width::Columns(18)] {
        assert_eq!(
            render(&tree, MarkdownDialect::MarkdownPlus, width),
            render(&tree, MarkdownDialect::Markdown, width)
        );
    }
}

#[test]
fn a_break_that_would_start_a_list_moves_earlier() {
    let tree = paragraph("one two three - four");
    let out = render(&tree, MarkdownDialect::Markdown, Width::Columns(14));
    assert_eq!(out, "one two\nthree - four");
}

#[test]
fn a_code_span_may_start_a_line() {
    let tree = RenderNode::root(vec![RenderNode::paragraph(vec![
        RenderNode::text("see the docs at "),
        RenderNode::inline_code("cargo doc --open"),
        RenderNode::text(" then "),
        RenderNode::inline_code("a``b"),
    ])]);
    let out = render(&tree, MarkdownDialect::Markdown, Width::Columns(20));
    assert_eq!(out, "see the docs at\n`cargo doc --open`\nthen ```a``b```");
    assert_wraps_faithfully(&tree, 1..=30);
}
