# Compose & Render Pipeline Benchmarks — Design

**Date:** 2026-05-16
**Package:** `darkmatter/lib`

## Goal

Add Criterion benchmarks that isolate each compose-pipeline stage and each
render mode, so an upcoming refactor's before/after `change:` delta points at
the specific stage that moved. The refactor spans both the compose and render
pipelines.

## File Organization

Two new bench files, mirroring the existing `benches/schema_validation.rs`
convention — one `[[bench]]` target per concern, `harness = false`, each file
self-contained with no shared module:

- `benches/compose_pipeline.rs`
- `benches/render_pipeline.rs`

Two `[[bench]]` entries are added to `lib/Cargo.toml`. `criterion 0.5`
(with `html_reports`) is already a dev-dependency — no Cargo dependency changes
beyond the bench registrations.

## Corpus

Each bench file has its own deterministic in-code corpus generator (modeled on
`schema_validation.rs`'s `build_corpus`), producing a fixed-size collection.
Synthetic generation keeps inputs frozen so the Criterion delta stays
meaningful across runs — no dependency on `example-docs/` or on-disk fixtures.

- **Compose corpus** — each generated document packs every benchmarked feature:
  frontmatter variables, `{{ }}` interpolation expressions, nested `::block`
  conditional regions, deliberately messy tables/spacing, and skewed heading
  levels. Contains **no** shell directives and **no** `::file` / `::code` /
  `::toc-linking` transclusion directives.
- **Render corpus** — each generated document packs headings, lists, tables,
  and multiple fenced code blocks (so syntax highlighting does real work).

Sizes are tuned so each benchmark run stays in a reasonable range: the compose
corpus is 200 documents; the render corpus is 30 documents (terminal rendering
is expensive) with a 20-document code-heavy sub-corpus for `highlight`.

## benches/compose_pipeline.rs

Per-stage benchmarks, each running `Markdown::compose_with` with
`ComposeOptions::only(&[op])` over the shared compose corpus:

| Benchmark        | `ComposeOperation`            |
|------------------|-------------------------------|
| `interpolation`  | `Interpolation`               |
| `page_blocks`    | `PageBlocks`                  |
| `cleanup`        | `Cleanup`                     |
| `normalization`  | `Normalization`               |
| `full_pipeline`  | default operation set         |

`full_pipeline` uses the default operations; shell-expansion and transclusion
operations are present in the default set but act as no-ops because the corpus
contains no such directives.

- Uses `compose_with` (immutable; returns a fresh `Markdown` + `ComposeReport`)
  so every iteration starts from a clean input.
- `Throughput::Elements` over the corpus length.

## benches/render_pipeline.rs

Per-mode benchmarks over the render corpus:

| Benchmark   | Entry point                                   |
|-------------|-----------------------------------------------|
| `parse`     | `Markdown::from(&str)` — parse cost, isolated  |
| `terminal`  | `for_terminal(&md, TerminalOptions::default())`|
| `html`      | `as_html(&md, HtmlOptions::default())`         |
| `highlight` | `for_terminal` over a code-block-heavy subset  |

- Terminal rendering pins `TerminalOptions::max_width` to 120 rather than
  relying on live terminal-size detection, so results are stable across runs.
- `highlight` isolates `syntect` syntax-highlighting cost by rendering the
  code-block-heavy subset of the corpus.

## Delta Workflow

Criterion automatically saves baselines under `target/criterion` and prints a
`change:` block on the next run. The explicit baseline form for a refactor:

```bash
# before refactor
cargo bench -p darkmatter --bench compose_pipeline -- --save-baseline before
cargo bench -p darkmatter --bench render_pipeline  -- --save-baseline before
# ...refactor...
cargo bench -p darkmatter --bench compose_pipeline -- --baseline before
cargo bench -p darkmatter --bench render_pipeline  -- --baseline before
```

This usage will be documented alongside the benchmarks (bench file doc comments
and/or `lib/README.md`).

## Out of Scope

- Shell expansion benchmarks (non-deterministic subprocess timing).
- Transclusion benchmarks (`::file` / `::code` / `::toc-linking`; require an
  on-disk fixture tree).
- Layout / `DarkmatterPage` benchmarks.
- Diff / document-comparison benchmarks.
