//! Side-by-side legacy-vs-tree benchmarks for the darkmatter render-tree
//! migration (DMTR-6).
//!
//! Every patch that shifts a public render path from the legacy
//! event-stream serializers onto the fold-then-render-tree pipeline must be
//! measurable with one repeatable `cargo bench` invocation. This file is the
//! measurement stick — it does not optimize, it does not pass/fail in code;
//! it just produces Criterion's `change:` numbers so a human can judge
//! whether a regression is acceptable for the migration stage.
//!
//! See `renderable/features/2026-05-20-darkmatter-tree/benchmark-harness-shape.md`
//! for the design.
//!
//! ## Run
//!
//! ```text
//! cargo bench -p darkmatter --bench migration_parity
//! ```
//!
//! Before/after a migration patch, persist a baseline:
//!
//! ```text
//! cargo bench -p darkmatter --bench migration_parity -- --save-baseline pre-migration
//! cargo bench -p darkmatter --bench migration_parity -- --baseline pre-migration
//! ```

use std::hint::black_box;
use std::rc::Rc;

use biscuit_terminal::discovery::detection::ColorDepth as TerminalColorDepth;
use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
use biscuit_terminal::terminal::Terminal;
use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::output::terminal::{ColorDepth, TerminalImageMode};
use darkmatter::markdown::output::{HtmlOptions, TerminalOptions, as_html, for_terminal};
use darkmatter::markdown::render_tree::{
    TerminalCodeRenderer, fold_markdown_spanned_with_frontmatter, fold_markdown_to_document,
};
use pulldown_cmark::{Options, Parser};
use renderable::tree::{
    BrowserRenderOptions, MarkdownRenderOptions, RawHtmlPolicy, RenderStrictness, SourceDescriptor,
    render_browser_document_html, render_markdown_document,
};

// ---------------------------------------------------------------------------
// Stress-input generators — mirrored from `benches/render_tree.rs` so this
// harness is self-contained.
// ---------------------------------------------------------------------------

fn large_code_block() -> String {
    let mut out = String::from("# Large code block\n\n```rust\n");
    for line in 0..600 {
        out.push_str(&format!(
            "fn generated_{line}() -> usize {{ let value = {line}; value * 2 }}\n"
        ));
    }
    out.push_str("```\n");
    out
}

fn large_table() -> String {
    let columns = 8;
    let rows = 200;
    let mut out = String::from("# Large table\n\n");
    out.push('|');
    for col in 0..columns {
        out.push_str(&format!(" Column {col} |"));
    }
    out.push('\n');
    out.push('|');
    for _ in 0..columns {
        out.push_str(" --- |");
    }
    out.push('\n');
    for row in 0..rows {
        out.push('|');
        for col in 0..columns {
            out.push_str(&format!(" r{row}c{col} cell |"));
        }
        out.push('\n');
    }
    out
}

fn deeply_nested_lists() -> String {
    let depth = 14;
    let siblings = 3;
    let mut out = String::from("# Nested lists\n\n");
    for level in 0..depth {
        let indent = "  ".repeat(level);
        for sibling in 0..siblings {
            out.push_str(&format!(
                "{indent}- level {level} item {sibling} with descriptive text\n"
            ));
        }
    }
    out
}

fn many_links_images() -> String {
    let mut out = String::from("# Links and images\n\n");
    for paragraph in 0..120 {
        out.push_str(&format!(
            "Paragraph {paragraph} references \
             [link {paragraph}](https://example.com/page/{paragraph} \"Title {paragraph}\") \
             and shows ![alt {paragraph}](https://cdn.example.com/img/{paragraph}.png) inline.\n\n"
        ));
    }
    out
}

/// A small prose document — short paragraphs and inline emphasis. Anchors
/// the lower end of the corpus so the `fold_once_multi_target` group has a
/// realistic small-input measurement.
fn small_prose() -> String {
    "# Small prose\n\n\
     This is a short paragraph of plain prose with *emphasis* and **strong** \
     words. A second sentence keeps the line count modest.\n\n\
     Another short paragraph closes the document.\n"
        .to_string()
}

/// A large kitchen-sink prose document — a synthetic README that exercises
/// most CommonMark + darkmatter constructs (headings, paragraphs, inline
/// emphasis / strong / code, links, images, ordered + unordered lists,
/// blockquotes, fenced code blocks, tables, and horizontal rules) without
/// any single construct dominating the workload. Anchors the
/// representative-workload end of the corpus, complementing the focused
/// stress fixtures (`large_code_block`, `large_table`, etc.) that isolate
/// one construct each.
///
/// Per `../../../renderable/features/2026-05-21-isolated-perf/spec.md`, the
/// existing fixtures answered "which feature regressed?" well but did not
/// give the parity ledger a typical-document anchor. This fixture closes
/// that gap.
fn large_prose() -> String {
    const SECTIONS: usize = 10;
    let mut out = String::from(
        "# Large prose\n\n\
         This document is a synthetic README built to exercise most rendering \
         features in a single fixture. It mixes *emphasis*, **strong**, \
         `inline code`, [links](https://example.com/intro), and \
         ![inline images](https://cdn.example.com/img/intro.png) so the \
         inline dispatcher stays warm across the whole pass.\n\n",
    );
    for section in 0..SECTIONS {
        out.push_str(&format!("## Section {section}\n\n"));
        out.push_str(&format!(
            "Section {section} opens with a paragraph carrying *emphasis*, \
             **strong** accents, `inline code`, a \
             [link to section {section}](https://example.com/section/{section} \"Section {section}\"), \
             and an inline ![image {section}](https://cdn.example.com/img/sec{section}.png).\n\n"
        ));
        if section % 2 == 0 {
            for item in 0..4 {
                out.push_str(&format!(
                    "- Unordered item {item} for section {section} with *em* and `code`\n"
                ));
            }
            out.push('\n');
        } else {
            for item in 1..=4 {
                out.push_str(&format!(
                    "{item}. Ordered item {item} for section {section} with **strong** and `code`\n"
                ));
            }
            out.push('\n');
        }
        match section % 4 {
            0 => {
                out.push_str(&format!(
                    "> Blockquote for section {section}. A short cited line with *emphasis* \
                     and a [reference](https://example.com/quote/{section}).\n\n"
                ));
            }
            1 => {
                out.push_str("```rust\n");
                out.push_str(&format!(
                    "fn section_{section}() -> &'static str {{\n    \"section {section} sample\"\n}}\n"
                ));
                out.push_str("```\n\n");
            }
            2 => {
                out.push_str("| Key | Value | Notes |\n");
                out.push_str("| --- | --- | --- |\n");
                out.push_str(&format!(
                    "| section | {section} | introduces the row |\n\
                     | count | {} | derived |\n\
                     | active | true | flag |\n\n",
                    section * 3
                ));
            }
            _ => {
                out.push_str("---\n\n");
            }
        }
        out.push_str(&format!(
            "Section {section} closes with a wrap-up paragraph that summarizes \
             the preceding constructs and keeps paragraph bookkeeping \
             representative of a real document.\n\n"
        ));
    }
    out.push_str(
        "## Conclusion\n\n\
         The trailing section closes the synthetic README with a final \
         paragraph so the document ends on prose rather than a block \
         construct.\n",
    );
    out
}

/// A document dominated by darkmatter-inline constructs — `==mark==`,
/// `⌄dim⌄`, and styled horizontal rules. Drives the span-aware fold so the
/// migration benches reflect realistic darkmatter feature usage.
fn mark_dim_hr() -> String {
    let mut out = String::from("# Darkmatter inline corpus\n\n");
    for paragraph in 0..80 {
        out.push_str(&format!(
            "Paragraph {paragraph}: a sentence with \
             ==highlighted phrase {paragraph}== and \u{2304}dimmed phrase {paragraph}\u{2304} \
             interleaved with plain prose.\n\n"
        ));
        if paragraph % 4 == 0 {
            out.push_str("--- { style: waves, weight: thick }\n\n");
        }
    }
    out
}

/// A document dominated by Markdown images and image-style references.
/// Stresses the image / link inline dispatcher; mirrors the spec's
/// image / Mermaid fixture category without requiring real Mermaid input
/// (the experimental tree path does not yet render Mermaid).
fn image_heavy() -> String {
    let mut out = String::from("# Image heavy\n\n");
    for paragraph in 0..80 {
        out.push_str(&format!(
            "Image block {paragraph}: ![alt {paragraph}](https://cdn.example.com/img/{paragraph}.png \"Title {paragraph}\")\n\n\
             Caption paragraph {paragraph} with [a link](https://example.com/{paragraph}) and \
             ![another image {paragraph}](https://cdn.example.com/img/{paragraph}b.png).\n\n"
        ));
    }
    out
}

/// Builds the fixture set used across every group in this harness.
///
/// Covers the corpus categories called out in
/// `renderable/features/2026-05-20-darkmatter-tree/spec.md` DMTR-6: small
/// prose, large prose, table-heavy, code-heavy, mark/dim/HR attributes,
/// image-heavy, and deeply-nested lists. Transclusion-heavy composed
/// documents are exercised by [`bench_full_pipeline`] separately because
/// they need to pass through `compose_with` first.
fn fixtures() -> Vec<(&'static str, String)> {
    vec![
        ("small_prose", small_prose()),
        ("large_prose", large_prose()),
        ("large_code_block", large_code_block()),
        ("large_table", large_table()),
        ("deeply_nested_lists", deeply_nested_lists()),
        ("many_links_images", many_links_images()),
        ("mark_dim_hr", mark_dim_hr()),
        ("image_heavy", image_heavy()),
    ]
}

// ---------------------------------------------------------------------------
// Pinned option helpers
// ---------------------------------------------------------------------------

/// Returns a [`TerminalOptions`] pinned for repeatable benchmarking with a
/// fixed width and explicit color depth. `Auto` color depth detection would
/// vary depending on host capability and would invalidate the baseline.
fn pinned_terminal_options(color: ColorDepth) -> TerminalOptions {
    let mut opts = TerminalOptions::default();
    opts.max_width = Some(120);
    opts.color_depth = Some(color);
    opts.image_mode = TerminalImageMode::Never;
    opts
}

/// Returns [`TerminalRenderOptions`] paired to the pinned terminal options
/// above, using an optimistic 120-wide [`Terminal`] whose `color_depth`
/// matches the legacy side. Closes review-5 finding 1: previously the
/// helper always produced a [`TerminalColorDepth::TrueColor`] context, so
/// the `migration/terminal_no_color` group was not actually measuring a
/// no-color tree path.
///
/// The [`TerminalCodeRenderer`] hook is wired in so the benchmarked tree
/// terminal path measures darkmatter's syntax-highlighted code-block cost —
/// matching the production entry point — rather than the render tree's
/// plain-fence fallback (review-10 finding 2). The `large_code_block`
/// fixture in particular would otherwise understate the tree path's real
/// per-render cost.
///
/// Mirrors the production entry point's `image_mode` mapping: `Auto` →
/// `GraphicsMode::Rich`.
fn pinned_tree_terminal_options(color: ColorDepth) -> TerminalRenderOptions {
    use renderable::tree::GraphicsMode;

    let mut term = Terminal::new_optimistic(120);
    term.color_depth = match color {
        ColorDepth::TrueColor => TerminalColorDepth::TrueColor,
        ColorDepth::Colors256 => TerminalColorDepth::Enhanced,
        ColorDepth::Colors16 => TerminalColorDepth::Basic,
        ColorDepth::None => TerminalColorDepth::None,
    };
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.graphics_mode = GraphicsMode::Rich;
    TerminalRenderOptions {
        context,
        strictness: RenderStrictness::Warn,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    }
}

/// Returns [`HtmlOptions`] used by both legacy and tree HTML benchmarks.
fn pinned_html_options() -> HtmlOptions {
    HtmlOptions::default()
}

/// Returns [`BrowserRenderOptions`] used by the tree HTML benchmark.
///
/// The [`TerminalCodeRenderer`] hook is wired in so the benchmarked tree HTML
/// path matches the production `render_tree_html` entry point (syntax-
/// highlighted code blocks) rather than the plain `<pre><code>` fallback.
///
/// Mirrors the production entry point's `mermaid_mode` mapping (`Off` →
/// `BrowserMermaidMode::Code`).
fn pinned_browser_options() -> BrowserRenderOptions {
    use renderable::tree::BrowserMermaidMode;

    BrowserRenderOptions {
        strictness: RenderStrictness::Warn,
        raw_html: RawHtmlPolicy::Escape,
        page: None,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
        mermaid_mode: BrowserMermaidMode::Code,
        ..Default::default()
    }
}

/// Builds a virtual [`SourceDescriptor`] keyed off the fixture name.
fn source_for(name: &str) -> SourceDescriptor {
    SourceDescriptor::Virtual { name: name.into() }
}

/// Folds a fixture into a [`Document`] through the **appropriate** tree-fold
/// for that fixture's content.
///
/// Fixtures whose names imply darkmatter-inline content (`==mark==`,
/// `⌄dim⌄`, HR-attribute paragraphs) must route through
/// [`fold_markdown_spanned_with_frontmatter`] so the recorded baseline
/// reflects the inline-span rewriter cost the spec's DMTR-6 corpus calls out
/// — closes review-4 finding 3. All other fixtures stay on the plain
/// [`fold_markdown_to_document`] path because that is what the legacy
/// terminal / HTML renderers consume.
///
/// [`Document`]: renderable::tree::Document
fn fold_fixture(
    name: &str,
    input: &str,
) -> (
    renderable::tree::Document,
    Vec<renderable::tree::Diagnostic>,
) {
    if uses_span_aware_fold(name) {
        let md: Markdown = input.into();
        fold_markdown_spanned_with_frontmatter(source_for(name), &md)
    } else {
        fold_markdown_to_document(source_for(name), input)
    }
}

/// Returns `true` for fixture names that contain darkmatter-inline content
/// the inline-span rewriter is responsible for.
fn uses_span_aware_fold(name: &str) -> bool {
    matches!(name, "mark_dim_hr")
}

// ---------------------------------------------------------------------------
// Benchmark groups
// ---------------------------------------------------------------------------

/// Terminal output, side by side: legacy `for_terminal` vs fold + render-tree
/// `render_terminal_document`.
fn bench_terminal(c: &mut Criterion) {
    let fixtures = fixtures();
    let term_opts = pinned_terminal_options(ColorDepth::TrueColor);
    let tree_opts = pinned_tree_terminal_options(ColorDepth::TrueColor);

    let mut group = c.benchmark_group("migration/terminal");
    group.sample_size(20);
    for (name, input) in &fixtures {
        let md: Markdown = input.as_str().into();
        group.bench_function(format!("{name}/legacy"), |b| {
            b.iter(|| {
                for_terminal(black_box(&md), term_opts.clone()).expect("legacy terminal render")
            })
        });
        group.bench_function(format!("{name}/tree"), |b| {
            b.iter(|| {
                let (doc, _diags) = fold_fixture(name, black_box(input));
                render_terminal_document(&doc, &tree_opts).expect("tree terminal render")
            })
        });
    }
    group.finish();
}

/// `ColorDepth::None` fast-path: legacy early-return vs the tree path. Pins
/// the no-color comparison that the spec calls out explicitly.
fn bench_terminal_no_color(c: &mut Criterion) {
    let fixtures = fixtures();
    let term_opts = pinned_terminal_options(ColorDepth::None);
    let tree_opts = pinned_tree_terminal_options(ColorDepth::None);
    debug_assert_eq!(
        tree_opts.context.terminal.color_depth,
        TerminalColorDepth::None,
        "migration/terminal_no_color must run with a ColorDepth::None tree context",
    );

    let mut group = c.benchmark_group("migration/terminal_no_color");
    group.sample_size(20);
    for (name, input) in &fixtures {
        let md: Markdown = input.as_str().into();
        group.bench_function(format!("{name}/legacy"), |b| {
            b.iter(|| {
                for_terminal(black_box(&md), term_opts.clone())
                    .expect("legacy no-color terminal render")
            })
        });
        group.bench_function(format!("{name}/tree"), |b| {
            b.iter(|| {
                let (doc, _diags) = fold_fixture(name, black_box(input));
                render_terminal_document(&doc, &tree_opts).expect("tree no-color terminal render")
            })
        });
    }
    group.finish();
}

/// Browser/HTML output, side by side: legacy `as_html` vs fold + render-tree
/// `render_browser_document_html` (the direct final-string renderer).
///
/// Both arms measure the same production surface: a final HTML `String`. The
/// legacy `as_html` already returns a `String`; the tree arm uses the direct
/// document-string renderer ([`render_browser_document_html`]) — the same path
/// the production `render_tree_html` entry point now takes. This streams HTML
/// into one buffer instead of building an intermediate `HtmlPage` /
/// `BrowserFragment` tree and serializing it, so the gate measures the path
/// production actually pays (see `2026-06-03-browser-perf/spec.md` §4 "the
/// browser gate under-measures the tree path").
fn bench_browser(c: &mut Criterion) {
    let fixtures = fixtures();
    let html_opts = pinned_html_options();
    let browser_opts = pinned_browser_options();

    report_browser_byte_sizes(&fixtures, &html_opts, &browser_opts);

    let mut group = c.benchmark_group("migration/browser");
    group.sample_size(20);
    for (name, input) in &fixtures {
        let md: Markdown = input.as_str().into();
        group.bench_function(format!("{name}/legacy"), |b| {
            b.iter(|| as_html(black_box(&md), html_opts.clone()).expect("legacy as_html"))
        });
        group.bench_function(format!("{name}/tree"), |b| {
            b.iter(|| {
                let (doc, _diags) = fold_fixture(name, black_box(input));
                black_box(
                    render_browser_document_html(&doc, &browser_opts)
                        .expect("tree browser render")
                        .output,
                )
            })
        });
    }
    group.finish();
}

/// Prints a per-fixture legacy/tree HTML byte-length diagnostic to stderr once
/// per `bench_browser` run.
///
/// The byte sizes settle the "fidelity vs structural overhead" question the
/// browser-perf spec raises before any profiling: equal byte length ⇒ the same
/// output produced by a slower path (pure overhead); a larger tree string ⇒ the
/// tree path emits extra markup (classes / `data-*` / provenance) that is
/// heavier and arguably higher-fidelity. It is a diagnostic, not a measurement —
/// it runs once, outside the timed `b.iter` loops.
fn report_browser_byte_sizes(
    fixtures: &[(&'static str, String)],
    html_opts: &HtmlOptions,
    browser_opts: &BrowserRenderOptions,
) {
    eprintln!("\n[migration/browser] per-fixture HTML byte sizes (legacy vs tree):");
    eprintln!("  {:<22} {:>12} {:>12} {:>10}", "fixture", "legacy", "tree", "tree/legacy");
    for (name, input) in fixtures {
        let md: Markdown = input.as_str().into();
        let legacy_len = as_html(&md, html_opts.clone())
            .expect("legacy as_html")
            .len();
        let (doc, _diags) = fold_fixture(name, input);
        let tree_len = render_browser_document_html(&doc, browser_opts)
            .expect("tree browser render")
            .output
            .len();
        let ratio = tree_len as f64 / legacy_len as f64;
        eprintln!("  {name:<22} {legacy_len:>12} {tree_len:>12} {ratio:>10.3}");
    }
    eprintln!();
}

/// Markdown round-trip output. There is no legacy Markdown roundtrip in
/// darkmatter, so this group only measures the tree path; it still lives in
/// the same harness so the full pipeline cost is captured next to the others.
fn bench_markdown(c: &mut Criterion) {
    let fixtures = fixtures();
    let opts = MarkdownRenderOptions::default();

    let mut group = c.benchmark_group("migration/markdown");
    group.sample_size(20);
    for (name, input) in &fixtures {
        group.bench_function(format!("{name}/tree"), |b| {
            b.iter(|| {
                let (doc, _diags) = fold_fixture(name, black_box(input));
                render_markdown_document(&doc, &opts).expect("tree markdown render")
            })
        });
    }
    group.finish();
}

/// `fold_only`: the cost of getting to a tree without rendering, paired
/// against draining the legacy inline-processor stack without target output.
fn bench_fold_only(c: &mut Criterion) {
    let fixtures = fixtures();
    let parser_options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;

    let mut group = c.benchmark_group("migration/fold_only");
    group.sample_size(20);
    for (name, input) in &fixtures {
        group.bench_function(format!("{name}/legacy"), |b| {
            // Drain the legacy `pulldown-cmark` parser stack without writing
            // target output. `preprocess_escaped_markers`,
            // `InlineStyleProcessor`, and `RuleProcessor` are crate-private,
            // so the bench measures the parser walk alone. That's the
            // dominant cost; this is a measurement stick, not an aggregate
            // optimization target.
            b.iter(|| {
                for event in Parser::new_ext(black_box(input), parser_options) {
                    let _ = black_box(event);
                }
            })
        });
        group.bench_function(format!("{name}/tree"), |b| {
            b.iter(|| {
                let (doc, diags) = fold_fixture(name, black_box(input));
                black_box((doc, diags))
            })
        });
    }
    group.finish();
}

/// `fold_production`: folds **every** fixture through the production span-aware
/// fold ([`fold_markdown_spanned_with_frontmatter`]) — the path the public
/// `to_render_document` entry point always takes.
///
/// [`bench_fold_only`] routes only `mark_dim_hr` through the span-aware fold and
/// every other fixture through the plain [`fold_markdown_to_document`], which
/// matches what the *legacy* renderers consume but hides the source-rewrite
/// scan cost the production tree path pays on ordinary, no-inline documents.
/// This group exposes that cost: each no-inline fixture here pays the
/// `rewrite_inline_extensions` scan (and, when a candidate delimiter is found,
/// the protected-region pre-parse) exactly as production does. Closes review-1
/// finding 4.
fn bench_fold_production(c: &mut Criterion) {
    let fixtures = fixtures();

    let mut group = c.benchmark_group("migration/fold_production");
    group.sample_size(20);
    for (name, input) in &fixtures {
        let md: Markdown = input.as_str().into();
        group.bench_function(*name, |b| {
            b.iter(|| {
                let (doc, diags) =
                    fold_markdown_spanned_with_frontmatter(source_for(name), black_box(&md));
                black_box((doc, diags))
            })
        });
    }
    group.finish();
}

/// `full_pipeline`: compose + render, side by side. Uses
/// [`compose_with`](Markdown::compose_with) with default options so the
/// composition cost is included on both sides.
fn bench_full_pipeline(c: &mut Criterion) {
    let fixtures = fixtures();
    let term_opts = pinned_terminal_options(ColorDepth::TrueColor);
    let tree_opts = pinned_tree_terminal_options(ColorDepth::TrueColor);

    let mut group = c.benchmark_group("migration/full_pipeline");
    group.sample_size(20);
    for (name, input) in &fixtures {
        group.bench_function(format!("{name}/legacy"), |b| {
            b.iter(|| {
                let md: Markdown = black_box(input.as_str()).into();
                let (composed, _report) = md
                    .compose_with(ComposeOptions::default())
                    .expect("legacy compose");
                for_terminal(&composed, term_opts.clone()).expect("legacy terminal render")
            })
        });
        group.bench_function(format!("{name}/tree"), |b| {
            b.iter(|| {
                let md: Markdown = black_box(input.as_str()).into();
                let (composed, _report) = md
                    .compose_with(ComposeOptions::default())
                    .expect("tree compose");
                let (doc, _diags) = fold_fixture(name, composed.content());
                render_terminal_document(&doc, &tree_opts).expect("tree terminal render")
            })
        });
    }
    group.finish();
}

/// `fold_once_multi_target`: parse + fold a document once, then render all
/// three render-tree targets (terminal, browser, markdown) from that single
/// document. This is the tree pipeline's strongest performance case per the
/// spec's "Performance Expectations" — the legacy path has no equivalent
/// because each legacy renderer re-parses the source from scratch.
///
/// The legacy side serializes the same three targets but pays the parse +
/// inline-processor cost three times, once per target, so the comparison
/// captures the architectural payoff that motivates the migration.
fn bench_fold_once_multi_target(c: &mut Criterion) {
    let fixtures = fixtures();
    let term_opts = pinned_terminal_options(ColorDepth::TrueColor);
    let tree_term_opts = pinned_tree_terminal_options(ColorDepth::TrueColor);
    let html_opts = pinned_html_options();
    let browser_opts = pinned_browser_options();
    let md_opts = MarkdownRenderOptions::default();

    let mut group = c.benchmark_group("migration/fold_once_multi_target");
    group.sample_size(15);
    for (name, input) in &fixtures {
        let md: Markdown = input.as_str().into();
        // Legacy: three separate render calls, each with its own parse.
        group.bench_function(format!("{name}/legacy"), |b| {
            b.iter(|| {
                let t = for_terminal(black_box(&md), term_opts.clone()).expect("legacy terminal");
                let h = as_html(black_box(&md), html_opts.clone()).expect("legacy html");
                // No legacy Markdown round-trip — keep parity by rendering
                // HTML twice so the legacy work is comparable in shape to
                // the tree side's three renderers.
                let h2 = as_html(black_box(&md), html_opts.clone()).expect("legacy html 2");
                black_box((t, h, h2));
            })
        });
        // Tree: fold once, render all three targets from the single document.
        group.bench_function(format!("{name}/tree"), |b| {
            b.iter(|| {
                let (doc, _diags) = fold_fixture(name, black_box(input));
                let t = render_terminal_document(&doc, &tree_term_opts).expect("tree terminal");
                // Render the browser output to the final HTML string through the
                // direct document-string renderer so this arm measures the same
                // production surface as the legacy `as_html` arm above (which
                // returns a `String`).
                let h = render_browser_document_html(&doc, &browser_opts)
                    .expect("tree browser")
                    .output;
                let m = render_markdown_document(&doc, &md_opts).expect("tree markdown");
                black_box((t, h, m));
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_terminal,
    bench_terminal_no_color,
    bench_browser,
    bench_markdown,
    bench_fold_only,
    bench_fold_production,
    bench_full_pipeline,
    bench_fold_once_multi_target,
);
criterion_main!(benches);
