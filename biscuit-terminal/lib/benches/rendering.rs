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
// Prose rendering
// ---------------------------------------------------------------------------

fn bench_prose_render(c: &mut Criterion) {
    let simple = "Hello world, this is a simple prose string.";
    let tokens = "Hello <bold>world</bold>! This is <red>important</red> text with <italic>emphasis</italic>.";
    let long_tokens = (0..20)
        .map(|i| format!("Item <bold>{i}</bold>: description of item {i} with <red>color</red>"))
        .collect::<Vec<_>>()
        .join(". ");

    let mut group = c.benchmark_group("prose_render");
    group.bench_function("simple_text", |b| {
        b.iter(|| {
            let prose = Prose::new(black_box(simple));
            prose.render_optimistic(Some(80))
        })
    });
    group.bench_function("with_tokens", |b| {
        b.iter(|| {
            let prose = Prose::new(black_box(tokens));
            prose.render_optimistic(Some(80))
        })
    });
    group.bench_function("long_tokens_20", |b| {
        b.iter(|| {
            let prose = Prose::new(black_box(&long_tokens));
            prose.render_optimistic(Some(80))
        })
    });
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
