# Perf Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the comprehensive benchmark suite that instruments the tree-cutover perf gate, add the minimal `DarkmatterPage` browser render path it requires, and capture the pre-cutover baseline.

**Architecture:** Criterion benches in two crates. `biscuit-terminal` gets one `component_render` group covering every tree-rendering component. `darkmatter` gets step-broken render-pipeline groups (terminal + browser), component benches (`YamlBlock`, `DarkmatterPage` on both targets), and a compose-pipeline coverage audit. The existing `migration_parity.rs` stays the bespoke-vs-tree comparison arm. (Correction during execution: `DarkmatterPage::render_to_browser` already exists, so Task C1 — "build a minimal browser path" — was dropped; the suite benches the existing method.)

**Tech Stack:** Rust, Criterion (`harness = false`), the `renderable::tree` renderers (`render_terminal_node`, `render_browser_document`), darkmatter fold helpers (`fold_markdown_spanned_with_frontmatter`).

**Spec:** [`spec.md`](./spec.md). **Reads:** [`../2026-06-02-tree-cutover/spec.md`](../2026-06-02-tree-cutover/spec.md) (Phase 1), [`../_completed/2026-05-20-darkmatter-tree/baselines.md`](../_completed/2026-05-20-darkmatter-tree/baselines.md).

---

## File Structure

- **Modify** `biscuit-terminal/lib/benches/render_tree.rs` — replace the degenerate `component_render_path_comparison` group with a `component_render` group covering every tree component.
- **Create** `darkmatter/lib/benches/render_pipeline_steps.rs` — new bench: `render_pipeline_terminal` + `render_pipeline_browser` step groups, plus `YamlBlock` and `DarkmatterPage` component groups. Registered in `darkmatter/lib/Cargo.toml`.
- ~~**Modify** `darkmatter/lib/src/layout/page.rs`~~ — DROPPED. `DarkmatterPage::render_to_browser` already exists (`pub`, tested, used by the CLI); no new browser method is needed.
- **Modify** `darkmatter/lib/benches/compose_pipeline.rs` — only if the coverage audit finds a missing stage (Task D1).
- **Modify** `darkmatter/lib/Cargo.toml` — register the new `[[bench]]`.
- **Append** `../_completed/2026-05-20-darkmatter-tree/baselines.md` — record the pre-cutover baseline (Task D3).

> **Bench verification convention.** Benches have no pass/fail assertions; "verify" means *compiles and runs*. Use `cargo bench -p <crate> --bench <name> --no-run` to verify compilation, and a short run (`-- <filter> --warm-up-time 1 --measurement-time 1 --sample-size 10`) to verify execution. Commit after each group compiles and runs.

---

## Part A — biscuit-terminal `component_render` suite

Replaces the degenerate `component_render_path_comparison` group (both arms route through the tree — see the tree-cutover baseline note) with a single tree-path `component_render` group covering every tree-rendering component.

### Task A1: Replace the comparison group with `component_render`

**Files:**
- Modify: `biscuit-terminal/lib/benches/render_tree.rs` (the `bench_component_render_path_comparison` fn + its `criterion_group!` entry)

- [ ] **Step 1: Read the existing bench to reuse its imports and `opts` setup**

Run: `sed -n '1,35p;242,335p' biscuit-terminal/lib/benches/render_tree.rs`
Expected: see the imports and the `bench_component_render_path_comparison` body (the constructors for `progress`, `unordered`, `ordered`, `section`, `two_column`, `table` are reused verbatim below).

- [ ] **Step 2: Replace `bench_component_render_path_comparison` with `bench_component_render`**

Replace the entire `fn bench_component_render_path_comparison(c: &mut Criterion) { … }` with:

```rust
/// Renders every tree-rendering component once, through the tree terminal
/// renderer (`render_tree_node()` → `render_terminal_node`). This is the
/// permanent per-component perf signal for the cutover gate (Part 2,
/// baseline-tracked). It is NOT a bespoke comparison — every listed component
/// already defaults to the tree, so there is no second arm to measure.
fn bench_component_render(c: &mut Criterion) {
    use biscuit_terminal::components::block_quote::BlockQuote;
    use biscuit_terminal::components::status_block::{StatusBlock, StatusState};
    use biscuit_terminal::components::text_block::TextBlock;
    use biscuit_terminal::components::todo::Todo;
    use biscuit_terminal::components::prose::Prose;

    let term = Terminal::new_optimistic(120);
    let opts = TerminalRenderOptions::new(&term, Default::default());

    // Reused constructors (verbatim from the retired comparison group).
    let progress = Progress::new(0.72).with_label("Indexing").with_bar_width(48);
    let unordered = UnorderedList::new(
        (0..80).map(|i| format!("Unordered item {i} with enough content to exercise wrapping")).collect::<Vec<_>>(),
    );
    let ordered = OrderedList::new(
        (0..80).map(|i| format!("Ordered step {i} with enough content to exercise wrapping")).collect::<Vec<_>>(),
    );
    let mut section = Section::new(HeadingLevel::h2, "Benchmark Section");
    for i in 0..60 {
        section.push(format!("Section paragraph {i} with component content projected into the render tree"));
    }
    let two_column = TwoColumn::new(
        "Left column\nwith multiple lines\nand repeated content",
        "Right column\nwith its own lines\nand repeated content",
    ).with_left_percent(0.45).with_gap(4);
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Status"), TableColumn::new("Notes")])
        .with_data(
            (0..80).map(|i| vec![
                TableCellContent::Text(format!("Item {i}")),
                TableCellContent::Text(if i % 3 == 0 { "Ready" } else { "Queued" }.into()),
                TableCellContent::Text(format!("This row has enough text to exercise wrapping {i}")),
            ]).collect(),
        );

    // New component coverage.
    let block_quote = BlockQuote::new(
        (0..20).map(|i| format!("Quoted line {i} with some length to wrap")).collect::<Vec<_>>().join("\n"),
    );
    let text_block = TextBlock::new(
        (0..40).map(|i| format!("Text block line {i} of uniformly styled content")).collect::<Vec<_>>().join("\n"),
    );
    let mut status_block = StatusBlock::new(StatusState::Info);
    status_block.push("A severity-colored status block body line with detail.");
    let todo = Todo::new("Wire the perf gate benchmark suite");
    let prose = Prose::new(
        "A <b>prose</b> paragraph with <red>color</red>, <i>emphasis</i>, and a \
         [link](https://example.com) plus enough words to exercise wrapping across the line.",
    );

    let mut group = c.benchmark_group("component_render");

    macro_rules! bench_component {
        ($name:literal, $component:expr) => {{
            group.bench_with_input(BenchmarkId::from_parameter($name), &$component, |b, component| {
                b.iter(|| {
                    let node = component.render_tree_node().expect("component supports tree rendering");
                    render_terminal_node(black_box(&node), black_box(&opts))
                        .expect("tree rendering should succeed")
                        .output
                })
            });
        }};
    }

    bench_component!("progress", progress);
    bench_component!("unordered_list_80", unordered);
    bench_component!("ordered_list_80", ordered);
    bench_component!("section_60", section);
    bench_component!("two_column", two_column);
    bench_component!("table_80x3", table);
    bench_component!("block_quote_20", block_quote);
    bench_component!("text_block_40", text_block);
    bench_component!("status_block", status_block);
    bench_component!("todo", todo);
    bench_component!("prose", prose);

    group.finish();
}
```

> `Compose` and `FileSystem` are intentionally omitted: `Compose` is a container measured indirectly by its children, and `FileSystem`'s terminal path is a Phase-3 holdout (still bespoke) and requires filesystem I/O, so it joins this suite when its terminal flip lands. Note this in the commit message.

- [ ] **Step 3: Update the `criterion_group!` registration**

Find `criterion_group!(benches, …, bench_component_render_path_comparison);` and rename the last entry to `bench_component_render`.

- [ ] **Step 4: Verify it compiles**

Run: `cargo bench -p biscuit-terminal --bench render_tree --no-run`
Expected: compiles. If a constructor signature differs (e.g. `StatusState` has no `Info` variant, or `BlockQuote::new` wants a different arg), fix to match the component's actual API — run `grep -n "enum StatusState" biscuit-terminal/lib/src/components/status_block.rs` and pick a real variant; the bench just needs *a* valid construction.

- [ ] **Step 5: Verify it runs**

Run: `cargo bench -p biscuit-terminal --bench render_tree -- component_render --warm-up-time 1 --measurement-time 1 --sample-size 10`
Expected: 11 `component_render/<name>` lines with `time:` output, no panics.

- [ ] **Step 6: Commit**

```bash
git add biscuit-terminal/lib/benches/render_tree.rs
git commit -m "perf(biscuit-terminal): add component_render suite, retire degenerate comparison group"
```

---

## Part B — darkmatter render-pipeline step benches

A new bench file with `parse` / `fold` / `render` / `full` broken out for each target, over a shared corpus. Reuses the building blocks `migration_parity.rs` already proves are bench-accessible.

### Task B1: Scaffold `render_pipeline_steps.rs` with the shared corpus and terminal group

**Files:**
- Create: `darkmatter/lib/benches/render_pipeline_steps.rs`
- Modify: `darkmatter/lib/Cargo.toml`

- [ ] **Step 1: Register the bench in `Cargo.toml`**

Add under the existing `[[bench]]` entries in `darkmatter/lib/Cargo.toml`:

```toml
[[bench]]
name = "render_pipeline_steps"
harness = false
```

- [ ] **Step 2: Create the bench with the shared corpus + terminal step group**

Create `darkmatter/lib/benches/render_pipeline_steps.rs`:

```rust
//! Step-broken render-pipeline benchmarks for the tree path (perf-gate Part 2).
//!
//! Each target's group isolates `parse` → `fold` → `render` and a `full`
//! end-to-end run, so a regression points at the stage that moved. The
//! bespoke-vs-tree comparison lives in `migration_parity.rs`; this file is
//! tree-only and baseline-tracked. Run:
//!
//! ```text
//! cargo bench -p darkmatter --bench render_pipeline_steps
//! ```

use std::hint::black_box;
use std::rc::Rc;

use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
use biscuit_terminal::terminal::Terminal;
use criterion::{Criterion, criterion_group, criterion_main};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::render_tree::{
    TerminalCodeRenderer, fold_markdown_spanned_with_frontmatter,
};
use renderable::tree::{
    BrowserRenderOptions, RawHtmlPolicy, RenderStrictness, SourceDescriptor,
    render_browser_document,
};

/// Mixed CommonMark + darkmatter corpus — headings, prose, lists, a table, a
/// code block, and darkmatter `==mark==` / `⌄dim⌄` so the fold's inline
/// rewriter is exercised.
fn corpus() -> String {
    let mut out = String::from("# Pipeline corpus\n\n");
    for s in 0..8 {
        out.push_str(&format!("## Section {s}\n\nParagraph with ==highlight {s}== and \u{2304}dim {s}\u{2304} and *emphasis*.\n\n"));
        out.push_str(&format!("- item {s}a\n- item {s}b\n\n"));
        if s % 2 == 0 {
            out.push_str("```rust\nfn x() -> usize { 1 }\n```\n\n");
        } else {
            out.push_str("| A | B |\n| --- | --- |\n| 1 | 2 |\n\n");
        }
    }
    out
}

fn source() -> SourceDescriptor {
    SourceDescriptor::Virtual { name: "pipeline_corpus".into() }
}

fn tree_terminal_options() -> TerminalRenderOptions {
    let term = Terminal::new_optimistic(120);
    TerminalRenderOptions {
        context: TerminalRenderContext::from_terminal(&term),
        strictness: RenderStrictness::Warn,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    }
}

fn browser_options() -> BrowserRenderOptions {
    BrowserRenderOptions {
        strictness: RenderStrictness::Warn,
        raw_html: RawHtmlPolicy::Escape,
        page: None,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    }
}

fn bench_render_pipeline_terminal(c: &mut Criterion) {
    let input = corpus();
    let term_opts = tree_terminal_options();
    let mut group = c.benchmark_group("render_pipeline_terminal");
    group.sample_size(20);

    group.bench_function("parse", |b| {
        b.iter(|| {
            let md: Markdown = black_box(input.as_str()).into();
            black_box(md)
        })
    });
    group.bench_function("fold", |b| {
        let md: Markdown = input.as_str().into();
        b.iter(|| {
            let (doc, diags) = fold_markdown_spanned_with_frontmatter(source(), black_box(&md));
            black_box((doc, diags))
        })
    });
    group.bench_function("render", |b| {
        let md: Markdown = input.as_str().into();
        let (doc, _d) = fold_markdown_spanned_with_frontmatter(source(), &md);
        b.iter(|| render_terminal_document(black_box(&doc), &term_opts).expect("terminal render"))
    });
    group.bench_function("full", |b| {
        b.iter(|| {
            let md: Markdown = black_box(input.as_str()).into();
            let (doc, _d) = fold_markdown_spanned_with_frontmatter(source(), &md);
            render_terminal_document(&doc, &term_opts).expect("terminal render")
        })
    });
    group.finish();
}

criterion_group!(benches, bench_render_pipeline_terminal);
criterion_main!(benches);
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo bench -p darkmatter --bench render_pipeline_steps --no-run`
Expected: compiles. If `TerminalRenderContext::from_terminal` / `BrowserRenderOptions` field names differ, cross-check against `darkmatter/lib/benches/migration_parity.rs` (which constructs the same types) and match exactly.

- [ ] **Step 4: Verify it runs**

Run: `cargo bench -p darkmatter --bench render_pipeline_steps -- render_pipeline_terminal --warm-up-time 1 --measurement-time 1 --sample-size 10`
Expected: `parse` / `fold` / `render` / `full` lines with `time:`.

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/benches/render_pipeline_steps.rs darkmatter/lib/Cargo.toml
git commit -m "perf(darkmatter): add step-broken render_pipeline_terminal bench"
```

### Task B2: Add the browser step group

**Files:**
- Modify: `darkmatter/lib/benches/render_pipeline_steps.rs`

- [ ] **Step 1: Add the browser group fn** (insert before `criterion_group!`)

```rust
fn bench_render_pipeline_browser(c: &mut Criterion) {
    let input = corpus();
    let browser_opts = browser_options();
    let mut group = c.benchmark_group("render_pipeline_browser");
    group.sample_size(20);

    group.bench_function("parse", |b| {
        b.iter(|| {
            let md: Markdown = black_box(input.as_str()).into();
            black_box(md)
        })
    });
    group.bench_function("fold", |b| {
        let md: Markdown = input.as_str().into();
        b.iter(|| {
            let (doc, diags) = fold_markdown_spanned_with_frontmatter(source(), black_box(&md));
            black_box((doc, diags))
        })
    });
    group.bench_function("render", |b| {
        let md: Markdown = input.as_str().into();
        let (doc, _d) = fold_markdown_spanned_with_frontmatter(source(), &md);
        b.iter(|| render_browser_document(black_box(&doc), &browser_opts).expect("browser render"))
    });
    group.bench_function("full", |b| {
        b.iter(|| {
            let md: Markdown = black_box(input.as_str()).into();
            let (doc, _d) = fold_markdown_spanned_with_frontmatter(source(), &md);
            render_browser_document(&doc, &browser_opts).expect("browser render")
        })
    });
    group.finish();
}
```

- [ ] **Step 2: Register it** — change `criterion_group!(benches, bench_render_pipeline_terminal);` to add `bench_render_pipeline_browser`.

- [ ] **Step 3: Verify compile + run**

Run: `cargo bench -p darkmatter --bench render_pipeline_steps -- render_pipeline_browser --warm-up-time 1 --measurement-time 1 --sample-size 10`
Expected: `parse`/`fold`/`render`/`full` browser lines.

- [ ] **Step 4: Commit**

```bash
git add darkmatter/lib/benches/render_pipeline_steps.rs
git commit -m "perf(darkmatter): add step-broken render_pipeline_browser bench"
```

---

## Part C — `DarkmatterPage` browser feature + component benches

### Task C1: ~~`DarkmatterPage::render_browser`~~ — DROPPED

> **Superseded during execution (2026-06-03).** `DarkmatterPage::render_to_browser`
> already exists — `pub`, tested, and used by the darkmatter CLI (`output.rs`).
> The premise that no browser path existed was wrong, so no new method is built.
> Task C2 benches the existing `render_to_browser`. The TDD detail below is
> retained only as a record of the abandoned approach.

### Task C1 (abandoned approach, for the record): `DarkmatterPage::render_browser`

**Files:**
- Modify: `darkmatter/lib/src/layout/page.rs` (new method + tests)

Minimal browser render: wrap the legacy HTML body in a `dm-page` `<div>`, centered by `max-width`, with a background class. Margin/padding browser mapping is deferred (terminal-cell concepts); the wrapper is faithful and benchable, matching how `render()` delegates the body to the legacy renderer (`for_terminal` → here `as_html`).

- [ ] **Step 1: Write the failing tests** (add to the `#[cfg(test)] mod tests` in `page.rs`)

```rust
#[test]
fn render_browser_wraps_body_in_dm_page_div() {
    let term = Terminal::new_optimistic(80);
    let md: Markdown = "# Title\n\nBody paragraph.".into();
    let html = DarkmatterPage::new(&term).render_browser(&md).unwrap();
    assert!(html.starts_with("<div class=\"dm-page\""), "got: {html}");
    assert!(html.contains("Body paragraph"), "body must be embedded: {html}");
    assert!(html.trim_end().ends_with("</div>"));
}

#[test]
fn render_browser_max_width_centers() {
    let term = Terminal::new_optimistic(80);
    let md: Markdown = "x".into();
    let html = DarkmatterPage::new(&term).with_max_width(72).render_browser(&md).unwrap();
    assert!(html.contains("max-width:72ch"), "got: {html}");
    assert!(html.contains("margin-left:auto;margin-right:auto"));
}

#[test]
fn render_browser_background_class() {
    let term = Terminal::new_optimistic(80);
    let md: Markdown = "x".into();
    let html = DarkmatterPage::new(&term)
        .with_page_background(PageBackground::Subtle)
        .render_browser(&md)
        .unwrap();
    assert!(html.contains("class=\"dm-page dm-page--subtle\""), "got: {html}");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p darkmatter --lib layout::page::tests::render_browser -- --nocapture`
Expected: FAIL — `no method named render_browser`.

- [ ] **Step 3: Implement `render_browser`** (add to `impl DarkmatterPage`, near `render`)

```rust
/// Renders the page to a minimal HTML wrapper around the document body.
///
/// The body is produced by the existing HTML renderer; the wrapper applies
/// the page's `max_width` (centered) and `page_background` as a `dm-page`
/// `<div>`. Margin/padding are terminal-cell concepts and are not mapped to
/// the browser here — this is the minimal page frame the perf-gate spec calls
/// for, not a browser layout engine.
///
/// ## Errors
///
/// Returns [`PageRenderError`] if the underlying HTML render fails.
pub fn render_browser(&self, md: &Markdown) -> Result<String, PageRenderError> {
    use crate::markdown::output::{HtmlOptions, as_html};

    let body = as_html(md, HtmlOptions::default()).map_err(PageRenderError::from)?;

    let mut classes = String::from("dm-page");
    match self.page_background() {
        PageBackground::Transparent => {}
        PageBackground::Subtle => classes.push_str(" dm-page--subtle"),
        PageBackground::Pronounced => classes.push_str(" dm-page--pronounced"),
    }
    let style = match self.max_width() {
        Some(w) => format!(" style=\"max-width:{w}ch;margin-left:auto;margin-right:auto\""),
        None => String::new(),
    };
    Ok(format!("<div class=\"{classes}\"{style}>{body}</div>"))
}
```

- [ ] **Step 4: Resolve the error conversion**

If `PageRenderError::from(MarkdownError)` does not exist, run `grep -n "enum PageRenderError\|impl.*From.*for PageRenderError" darkmatter/lib/src/layout/page.rs`. Add a `#[from]` variant if missing:

```rust
// in the PageRenderError enum definition:
/// The underlying HTML body render failed.
#[error("html body render failed: {0}")]
HtmlBody(#[from] crate::markdown::MarkdownError),
```

(Match the actual error type name from `as_html`'s `MarkdownResult` — `grep -n "type MarkdownResult" darkmatter/lib/src/markdown` to confirm the `Err` type.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p darkmatter --lib layout::page::tests::render_browser`
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add darkmatter/lib/src/layout/page.rs
git commit -m "feat(darkmatter): add minimal DarkmatterPage::render_browser page wrapper"
```

### Task C2: `YamlBlock` + `DarkmatterPage` component benches

**Files:**
- Modify: `darkmatter/lib/benches/render_pipeline_steps.rs`

- [ ] **Step 1: Add imports** (top of the bench file)

```rust
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::yaml_block::YamlBlock;
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
```

(Confirm the `YamlBlock` path: `grep -n "pub use\|pub mod yaml_block" darkmatter/lib/src/markdown/mod.rs` — adjust the `use` to the real public path.)

- [ ] **Step 2: Add the component group fn** (before `criterion_group!`)

```rust
fn bench_darkmatter_components(c: &mut Criterion) {
    let yaml = YamlBlock::new("name: example\nvalues:\n  - a\n  - b\n  - c\ncount: 3\nactive: true")
        .expect("valid yaml");
    let term = Terminal::new_optimistic(120);
    let page_md: Markdown =
        "# Page\n\nParagraph with *emphasis* and a list:\n\n- one\n- two\n\n```rust\nfn x() {}\n```\n".into();

    let mut group = c.benchmark_group("darkmatter_components");
    group.sample_size(20);

    group.bench_function("yaml_block/terminal", |b| {
        b.iter(|| TerminalRenderable::render(black_box(&yaml), &term))
    });
    group.bench_function("yaml_block/browser", |b| {
        b.iter(|| BrowserRenderable::render_html_fragment(black_box(&yaml)).render())
    });
    group.bench_function("darkmatter_page/terminal", |b| {
        b.iter(|| {
            DarkmatterPage::new(&term).with_max_width(100)
                .render(black_box(&page_md)).expect("page terminal render")
        })
    });
    group.bench_function("darkmatter_page/browser", |b| {
        b.iter(|| {
            DarkmatterPage::new(&term).with_max_width(100)
                .render_to_browser(black_box(&page_md)).expect("page browser render")
        })
    });
    group.finish();
}
```

- [ ] **Step 3: Register it** — add `bench_darkmatter_components` to `criterion_group!`.

- [ ] **Step 4: Verify compile + run**

Run: `cargo bench -p darkmatter --bench render_pipeline_steps -- darkmatter_components --warm-up-time 1 --measurement-time 1 --sample-size 10`
Expected: 4 lines (`yaml_block/terminal`, `yaml_block/browser`, `darkmatter_page/terminal`, `darkmatter_page/browser`). If `BrowserFragment` has no `.render()`, check `render_html_fragment`'s return type and use its rendering method (`grep -n "fn render" renderable/src/browser/fragment.rs`).

- [ ] **Step 5: Commit**

```bash
git add darkmatter/lib/benches/render_pipeline_steps.rs
git commit -m "perf(darkmatter): add YamlBlock and DarkmatterPage component benches"
```

---

## Part D — Compose audit + baseline capture

### Task D1: Audit compose-pipeline stage coverage

**Files:**
- Read: `darkmatter/lib/benches/compose_pipeline.rs`
- Reference: `darkmatter/lib/src/markdown/compose/` (stage list)

- [ ] **Step 1: List the benched stages vs the real pipeline**

Run: `grep -nE "ComposeOptions::only|bench_function|group.bench" darkmatter/lib/benches/compose_pipeline.rs`
Run: `grep -rnE "pub enum ComposeOperation|enum.*Stage|Operation::" darkmatter/lib/src/markdown/compose/ | head`
Expected: compare. The compose pipeline stages (per `darkmatter`'s skill): frontmatter interpolation, schema validation, frontmatter shell expansion, text replacement, page blocks, interpolation, shell expansion, link resolve, transclusion, inline-post.

- [ ] **Step 2: If a stage is missing AND benchable, add a `bench_function` for it**

If every `ComposeOptions::only(...)`-supported stage is already present (shell + transclusion are documented exclusions — they need I/O), no change is needed: record "coverage confirmed, shell/transclusion excluded by design" in the commit message and skip to Task D2.

If a stage is missing, add it following the existing per-stage pattern in the file (one `group.bench_function("<stage>", …)` calling `compose_with(ComposeOptions::only(<stage>))`).

- [ ] **Step 3: Verify**

Run: `cargo bench -p darkmatter --bench compose_pipeline --no-run`
Expected: compiles.

- [ ] **Step 4: Commit (only if changed)**

```bash
git add darkmatter/lib/benches/compose_pipeline.rs
git commit -m "perf(darkmatter): confirm/extend compose_pipeline stage coverage"
```

### Task D2: Full baseline run

- [ ] **Step 1: Run every gate bench and save the baseline**

```bash
cargo bench -p biscuit-terminal --bench render_tree -- --save-baseline pre-cutover-2026-06-02
cargo bench -p darkmatter --bench render_pipeline_steps -- --save-baseline pre-cutover-2026-06-02
cargo bench -p darkmatter --bench compose_pipeline -- --save-baseline pre-cutover-2026-06-02
cargo bench -p darkmatter --bench migration_parity -- --save-baseline pre-cutover-2026-06-02 \
    --warm-up-time 1 --measurement-time 3 --sample-size 10
```

Expected: each completes; Criterion writes baselines under `target/criterion/`.

- [ ] **Step 2: Capture the Part-1 ratios from `migration_parity`**

Run: `cargo bench -p darkmatter --bench migration_parity -- migration/browser --warm-up-time 1 --measurement-time 3 --sample-size 10 2>&1 | grep -A1 "migration/browser/.*/tree\|migration/browser/.*/legacy"`
Expected: capture the per-fixture legacy + tree middle estimates needed for the geomean and the 1.5× ceiling check (browser `large_table` is the known ceiling breach to flag).

### Task D3: Record the baseline in `baselines.md`

**Files:**
- Append: `renderable/features/_completed/2026-05-20-darkmatter-tree/baselines.md`

- [ ] **Step 1: Append a new dated section**

Add a `## Recorded Baselines (2026-06-02, perf-gate suite — pre-cutover)` section with: (a) the Part-1 `migration_parity` per-target geomean + the per-fixture ratios, flagging browser `large_table` as the ceiling breach (Decision #9 / perf-gate Part 1) to fix or except before Phase 5; (b) a note that the tree-only suites (`component_render`, `render_pipeline_*`, `darkmatter_components`, `compose_pipeline`) were saved as Criterion baseline `pre-cutover-2026-06-02` for the Part-2 >10% trend guard. Use the table format already in the file.

- [ ] **Step 2: Commit**

```bash
git add renderable/features/_completed/2026-05-20-darkmatter-tree/baselines.md
git commit -m "docs(perf): record pre-cutover perf-gate baseline"
```

---

## Self-Review

- **Spec coverage.** Part A → biscuit-terminal component suite (spec "Benchmark Inventory · biscuit-terminal"). Part B → render-pipeline terminal/browser step benches (spec items 2, 3). Part C1 → DarkmatterPage browser path (spec item 5 + "DarkmatterPage Browser Path"). Part C2 → YamlBlock terminal/browser (items 6, 7) + DarkmatterPage terminal (item 4). Part D1 → compose audit (item 1). Part D2/D3 → baseline capture + `baselines.md` (spec "Mechanics", gate Part 2). The gate *criterion* itself (geomean ≤ 1.0×, 1.5× ceiling, >10% trend) is documented in the spec and recorded in `baselines.md` (D3); enforcement runs at cutover Phase 4 — out of scope for this build plan.
- **Known API lookups flagged, not hidden.** Three spots (StatusState variant A4-step4, PageRenderError conversion C1-step4, YamlBlock/BrowserFragment paths C2) carry an exact `grep` to confirm the real API — these are real, bounded lookups, not deferred design.
- **Type consistency.** `fold_markdown_spanned_with_frontmatter`, `render_terminal_document`, `render_browser_document`, `TerminalRenderOptions`, `BrowserRenderOptions`, `TerminalCodeRenderer` are used exactly as `migration_parity.rs` uses them (cross-checked). C2 benches the existing `render_to_browser` (Task C1 dropped — the method already existed).
- **No degenerate comparison.** Part A removes the misleading `component_render_path_comparison`; `migration_parity` remains the sole bespoke-vs-tree arm.
