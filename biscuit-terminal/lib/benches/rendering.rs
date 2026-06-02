//! Criterion benchmarks for biscuit-terminal hot paths.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::pad::{PadLeft, PadRight};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::utils::escape_codes;
use biscuit_terminal::utils::layout::{Layout, LayoutTerminalExt, WordWrap};
use biscuit_terminal::utils::word_wrap::word_wrap;
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Escape code stripping
// ---------------------------------------------------------------------------

fn bench_strip_escape_codes(c: &mut Criterion) {
    let plain = "Hello, this is a plain text string with no escape codes at all.";
    let styled = "\x1b[1m\x1b[31mError:\x1b[0m Something went \x1b[4mwrong\x1b[24m in \x1b[38;2;100;200;50mmodule\x1b[0m";
    let heavy = (0..50)
        .map(|i| format!("\x1b[38;2;{i};{i};{i}mword{i}\x1b[0m"))
        .collect::<Vec<_>>()
        .join(" ");

    let mut group = c.benchmark_group("strip_escape_codes");
    group.bench_function("plain_text", |b| {
        b.iter(|| escape_codes::strip_escape_codes(black_box(plain)))
    });
    group.bench_function("styled_short", |b| {
        b.iter(|| escape_codes::strip_escape_codes(black_box(styled)))
    });
    group.bench_function("heavy_50_spans", |b| {
        b.iter(|| escape_codes::strip_escape_codes(black_box(heavy.as_str())))
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Prose rendering (cross-target hot path)
// ---------------------------------------------------------------------------
//
// Prose is the 132-file inline-text hot primitive. After its full collapse
// onto the shared render tree (`2026-06-02-prose-tree`) it renders to every
// target through the tree renderers, so its cost is tracked across terminal,
// browser, Markdown, and MarkdownPlus over three corpus shapes. The recorded
// baseline lives in
// `renderable/features/_completed/2026-05-20-darkmatter-tree/baselines.md`.
// Each bench measures the full parse + render hot path (`Prose::new` builds the
// `RenderNode` tree once, then the shared renderer folds it) — the shape real
// callers hit.

/// Small corpus: a single short CLI line with light styling — the common
/// one-liner status shape.
fn corpus_small() -> String {
    "Build <bold>complete</bold>: 3 passed, <red>1 failed</red>.".to_string()
}

/// Medium corpus: a multi-clause report block mixing weight, dim, color, and
/// a Markdown link per clause — a typical status/report paragraph.
fn corpus_medium() -> String {
    (0..12)
        .map(|i| {
            format!(
                "Item <bold>{i}</bold>: <dim>step {i}</dim> finished with \
                 <red>code {i}</red>, see [report](https://example.com/run/{i})"
            )
        })
        .collect::<Vec<_>>()
        .join(". ")
}

/// Tag-dense corpus: deeply nested spans, every emphasis / color / underline /
/// inverse variant, links, escaped literal markup, and a trailing fenced code
/// block — the adversarial maximum-work shape for the parser and the tree fold.
fn corpus_tag_dense() -> String {
    let mut parts: Vec<String> = (0..16)
        .map(|i| {
            format!(
                "<red><b>err{i}</b> <i>at</i> <u>line {i}</u></red> \
                 <inverse>HOT</inverse> <bg-rgb 1,2,3>note</bg-rgb> \
                 <a href=\"https://example.com/a_b/{i}\">link{i}</a> \
                 <dim>\\<ENV{i}\\></dim>"
            )
        })
        .collect();
    parts.push("```rust\nlet x = 1; // not a <tag>\n```".to_string());
    parts.join(" ")
}

fn bench_prose_render(c: &mut Criterion) {
    let corpora = [
        ("small", corpus_small()),
        ("medium", corpus_medium()),
        ("tag_dense", corpus_tag_dense()),
    ];

    let mut group = c.benchmark_group("prose_render");
    for (size, input) in &corpora {
        group.bench_function(format!("terminal_{size}"), |b| {
            b.iter(|| Prose::new(black_box(input.as_str())).render_optimistic(Some(80)))
        });
        group.bench_function(format!("browser_{size}"), |b| {
            b.iter(|| {
                Prose::new(black_box(input.as_str()))
                    .render_html_fragment()
                    .render()
            })
        });
        group.bench_function(format!("markdown_{size}"), |b| {
            b.iter(|| Prose::new(black_box(input.as_str())).render_markdown())
        });
        group.bench_function(format!("markdown_plus_{size}"), |b| {
            b.iter(|| Prose::new(black_box(input.as_str())).render_markdown_plus())
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Word wrapping
// ---------------------------------------------------------------------------

fn bench_word_wrap(c: &mut Criterion) {
    let short = "A short line that fits.";
    let paragraph = "This is a longer paragraph that will definitely need to be wrapped because it exceeds eighty columns when rendered in a typical terminal window and contains multiple clauses separated by conjunctions.";
    let multiline = "First line of content here.\nSecond line with more text that may wrap.\nThird line is short.\nFourth line has <bold>tokens</bold> interspersed throughout the content for styling.";

    let mut group = c.benchmark_group("word_wrap");
    group.bench_function("short_no_wrap", |b| {
        b.iter(|| word_wrap(black_box(short), WordWrap::WrapProse(None, None), 80))
    });
    group.bench_function("paragraph_wrap_80", |b| {
        b.iter(|| word_wrap(black_box(paragraph), WordWrap::WrapProse(None, None), 80))
    });
    group.bench_function("multiline_wrap_60", |b| {
        b.iter(|| word_wrap(black_box(multiline), WordWrap::WrapProse(None, None), 60))
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Layout application
// ---------------------------------------------------------------------------

fn bench_layout_apply(c: &mut Criterion) {
    let content = "This is some content that will have layout applied to it including margins.";
    let multiline = "Line one of the content.\nLine two with more text.\nLine three is shorter.";

    let mut group = c.benchmark_group("layout_apply");
    group.bench_function("default_layout", |b| {
        let layout = Layout::default();
        b.iter(|| layout.apply_layout(black_box(content), 80))
    });
    group.bench_function("with_margins", |b| {
        let layout = Layout {
            margin: biscuit_terminal::utils::layout::Margin::x(
                biscuit_terminal::utils::layout::Length::ch(4),
            ),
            ..Default::default()
        };
        b.iter(|| layout.apply_layout(black_box(content), 80))
    });
    group.bench_function("multiline_with_wrap", |b| {
        let layout = Layout {
            word_wrap: WordWrap::WrapProse(None, None),
            ..Default::default()
        };
        b.iter(|| layout.apply_layout(black_box(multiline), 60))
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Component rendering (quote, list, pad)
// ---------------------------------------------------------------------------

fn bench_component_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_render");

    group.bench_function("block_quote", |b| {
        b.iter(|| {
            let prose = Prose::new("To be or not to be, that is the question.");
            let quote = BlockQuote::new(
                RenderableTerminalContent::Component(Rc::new(prose)),
                Some("Shakespeare"),
            );
            quote.render_optimistic(Some(80))
        })
    });

    group.bench_function("unordered_list_5", |b| {
        b.iter(|| {
            let items: Vec<RenderableTerminalContent> = (1..=5)
                .map(|i| {
                    RenderableTerminalContent::Component(Rc::new(Prose::new(format!(
                        "Item number {i} with some descriptive text"
                    ))))
                })
                .collect();
            let list = UnorderedList::from(items);
            list.render_optimistic(Some(80))
        })
    });

    group.bench_function("pad_left", |b| {
        b.iter(|| {
            let prose = Prose::new("hello");
            let pad = PadLeft::new(prose, 30);
            pad.render_optimistic(Some(80))
        })
    });

    group.bench_function("pad_right", |b| {
        b.iter(|| {
            let prose = Prose::new("hello");
            let pad = PadRight::new(prose, 30);
            pad.render_optimistic(Some(80))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_strip_escape_codes,
    bench_prose_render,
    bench_word_wrap,
    bench_layout_apply,
    bench_component_render,
);
criterion_main!(benches);
