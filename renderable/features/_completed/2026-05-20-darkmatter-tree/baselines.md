# Migration Parity Baselines

Recorded baseline numbers for the darkmatter render-tree migration. These
numbers gate the public cutover decision (see `spec.md` DMTR-6 acceptance
criteria); a regression on any group needs a recorded explanation or fix
before the matching public target flips from the legacy renderer to the
tree pipeline.

## Capture Procedure

```bash
# Full bench suite (all groups, all fixtures). Takes several minutes.
cargo bench -p darkmatter --bench migration_parity

# Save as a Criterion baseline for trend tracking.
cargo bench -p darkmatter --bench migration_parity -- \
    --save-baseline pre-migration

# Compare a later run against the saved baseline.
cargo bench -p darkmatter --bench migration_parity -- \
    --baseline pre-migration
```

## Corpus Coverage

The benchmark harness exercises the corpus categories called out in
`spec.md` DMTR-6:

| Fixture                | Category                              |
|------------------------|---------------------------------------|
| `small_prose`          | Small prose document                  |
| `large_prose`          | Large prose document                  |
| `large_code_block`     | Code-heavy document                   |
| `large_table`          | Table-heavy document                  |
| `deeply_nested_lists`  | List-heavy document                   |
| `many_links_images`    | Link / image inline dispatcher        |
| `mark_dim_hr`          | Darkmatter `==mark==`, `⌄dim⌄`, `--- { ... }` |
| `image_heavy`          | Image / Mermaid-shaped inline dispatcher |

The `image_heavy` fixture stresses image-style references; Mermaid
specifically is deferred until the tree renderer gains a Mermaid adapter
(see `entry-point-shape.md` deferred list). Transclusion-heavy composed
documents are exercised by the `migration/full_pipeline` group through
`compose_with` rather than as a standalone fixture.

## Color Depth Selection

The bench harness's `pinned_tree_terminal_options(color)` builds the tree
side's terminal context with the **same** color depth as the legacy side's
`pinned_terminal_options(color)`. Previously the helper ignored its
argument and always built a `TrueColor` optimistic terminal, so the
`migration/terminal_no_color` group did not actually measure a `None`
tree context — see review-5 finding 1. A `debug_assert_eq!` in
`bench_terminal_no_color` now pins the tree context's `color_depth` to
`TerminalColorDepth::None` so the no-color comparison cannot silently
regress again.

The `migration/terminal_no_color` numbers in the tables below were
**re-captured** after this fix, so they reflect a real `ColorDepth::None`
tree context (not the TrueColor tree work the colored group measures).

## Code Renderer Wiring

The bench harness's `pinned_tree_terminal_options(color)` now wires
darkmatter's `TerminalCodeRenderer` into the tree terminal options (review-10
finding 2), matching the production `render_tree_terminal` entry point.
Previously the helper passed `code_renderer: None`, so the benchmarked tree
terminal path measured the render tree's plain dim-fence fallback instead of
darkmatter's syntax-highlighted code path.

`pinned_browser_options()` likewise wires the `TerminalCodeRenderer` (review-11
finding 2), so the tree HTML group now pays darkmatter's syntect cost — plus
info-string title / line-number / highlight handling via the `CodeRenderer`
`meta` parameter — instead of the plain `<pre><code>` fallback.

The `large_code_block` rows in the tables below were **re-captured** after
both fixes, so the tree terminal and tree HTML numbers reflect the
syntax-highlighting cost the production path now pays — not the earlier
plain-fence / `<pre><code>` fallback.

## Fold Selection

The bench harness routes each fixture through the **fold appropriate for
its content** so the recorded baselines reflect the production fold path:

- Fixtures whose names imply darkmatter-inline content (`==mark==`,
  `⌄dim⌄`, HR-attribute paragraphs — currently only `mark_dim_hr`) fold
  through [`fold_markdown_spanned_with_frontmatter`] so the span-aware
  processor cost is included.
- All other fixtures fold through [`fold_markdown_to_document`] because
  that is what the legacy renderers consume.

Earlier baselines that always used the plain fold understated the
`mark_dim_hr` tree cost — see review-4 finding 3. Numbers below reflect
the corrected routing.

[`fold_markdown_spanned_with_frontmatter`]: ../../../../darkmatter/lib/src/markdown/render_tree/fold.rs
[`fold_markdown_to_document`]: ../../../../darkmatter/lib/src/markdown/render_tree/fold.rs

## Recorded Baselines (2026-05-21, sample subset)

Captured on the development host with
`--warm-up-time 1 --measurement-time 3 --sample-size 10`. Times are
Criterion's middle estimate; the harness emits low/middle/high CIs for
every measurement in the full report. The `large_code_block` rows exercise
the wired `TerminalCodeRenderer` on both the tree terminal and tree HTML
paths (review-10 / review-11 finding 2); the `terminal_no_color` rows use a
real `ColorDepth::None` tree context (review-5 finding 1).

### `migration/terminal` (TrueColor)

| Fixture            | Legacy        | Tree         | Tree / Legacy |
|--------------------|---------------|--------------|---------------|
| `small_prose`      | 5.60 ms       | 15.60 µs     | ≈ 0.0028×     |
| `large_code_block` | 27.80 ms      | 20.35 ms     | ≈ 0.73×       |
| `mark_dim_hr`      | 5.93 ms       | 5.47 ms      | ≈ 0.92×       |

### `migration/terminal_no_color` (`ColorDepth::None`)

| Fixture            | Legacy        | Tree         | Tree / Legacy |
|--------------------|---------------|--------------|---------------|
| `small_prose`      | 98.96 ns      | 15.39 µs     | ≈ 155×        |
| `large_code_block` | 10.07 µs      | 489.47 µs    | ≈ 49×         |
| `mark_dim_hr`      | 3.18 µs       | 5.50 ms      | ≈ 1730×       |

The no-color group is the documented spec exception: legacy's
`ColorDepth::None` early return is a fast path with no equivalent on the
tree side. The tree path remains the chosen architecture; the regression
is accepted as the spec calls out under "Performance Expectations" and
will be revisited with a tree no-color fast path before public terminal
cutover. The `mark_dim_hr` ratio is the most extreme because that fixture's
20 HR-attribute rules each rasterize through the Tier-1 image path on the
tree side even when colors are off, while legacy's no-color rule output is
near-free; this is the same HR cost visible in the TrueColor group.

### `migration/browser`

| Fixture            | Legacy        | Tree         | Tree / Legacy |
|--------------------|---------------|--------------|---------------|
| `small_prose`      | 5.07 µs       | 10.69 µs     | ≈ 2.1×        |
| `large_code_block` | 17.97 ms      | 18.04 ms     | ≈ 1.00×       |
| `mark_dim_hr`      | 125.79 µs     | 756.36 µs    | ≈ 6.0×        |

### `migration/markdown`

The legacy renderer has no Markdown round-trip; only the tree path runs.

| Fixture            | Tree         |
|--------------------|--------------|
| `small_prose`      | 9.29 µs      |
| `large_code_block` | 12.16 µs     |
| `mark_dim_hr`      | 578.34 µs    |

### `migration/fold_only`

| Fixture            | Legacy        | Tree         | Tree / Legacy |
|--------------------|---------------|--------------|---------------|
| `small_prose`      | 493.17 ns     | 1.15 µs      | ≈ 2.34×       |
| `large_code_block` | 5.35 µs       | 6.70 µs      | ≈ 1.25×       |
| `mark_dim_hr`      | 9.04 µs       | 164.47 µs    | ≈ 18.2×       |

The `mark_dim_hr` fold ratio reflects the span-aware processor cost: the
fixture's 80 mark/dim paragraphs and 20 HR-attribute rules all route
through `fold_markdown_spanned_with_frontmatter`, where the legacy fold runs
only the plain event stream.

### `migration/fold_once_multi_target`

The tree pipeline's strongest case: parse + fold once, render all three
targets (terminal + browser + markdown) from the single document. The
legacy comparator renders three targets too, each of which re-parses.

| Fixture            | Legacy        | Tree         | Tree / Legacy |
|--------------------|---------------|--------------|---------------|
| `small_prose`      | 5.60 ms       | 33.65 µs     | ≈ 0.0060×     |
| `large_code_block` | 64.00 ms      | 38.09 ms     | ≈ 0.60×       |
| `mark_dim_hr`      | 6.36 ms       | 6.57 ms      | ≈ 1.03×       |

## Recorded Baselines (2026-06-01, `mark_dim_hr` after inline-span cutover)

Captured after the inline-span cutover (`2026-05-26-inline-span` Phase 6),
which deleted the per-event span-aware inline transport and routes
`fold_markdown_spanned_with_frontmatter` through the source-layer
`inline_extension` rewriter. Only the `mark_dim_hr` fixture was re-run, with
`--warm-up-time 1 --measurement-time 3 --sample-size 10`; the other fixtures'
rows above are unchanged. Times are Criterion's middle estimate.

| Group                        | Legacy     | Tree       | Tree / Legacy |
|------------------------------|------------|------------|---------------|
| `terminal` (TrueColor)       | 5.83 ms    | 4.44 ms    | ≈ 0.76×       |
| `terminal_no_color`          | 5.87 ms    | 4.45 ms    | ≈ 0.76×       |
| `browser`                    | 126.61 µs  | 652.04 µs  | ≈ 5.15×       |
| `markdown` (tree only)       | —          | 573.40 µs  | —             |
| `fold_only`                  | 9.05 µs    | 157.38 µs  | ≈ 17.4×       |
| `fold_once_multi_target`     | 5.99 ms    | 5.37 ms    | ≈ 0.90×       |

The `fold_only/tree` cost (157 µs) is the source-rewrite inline-span path that
replaced the deleted span-aware processor; it is marginally cheaper than the
prior processor's 164 µs row above, confirming the cutover carried no fold-cost
regression. The `browser` ratio is dominated by the `mark`/`dim` inline node
construction plus the `<mark>` recovery, not HR SVG generation (the tree path
emits a plain `<hr>`). On this capture the terminal `no_color` legacy row no
longer hits the old `ColorDepth::None` fast path the 2026-05-21 subset recorded
— the legacy and TrueColor terminal numbers now coincide — so that group is
reported as captured rather than compared against the earlier microsecond
figure.

## Recorded Baselines (2026-06-02, `migration/fold_production` production-path fold)

The 2026-06-01 section above only re-ran `mark_dim_hr`, which left the
production tree path's cost on **no-inline** documents unrecorded. The public
`to_render_document` entry point always routes through
`fold_markdown_spanned_with_frontmatter`, so every document — even one with no
`==mark==` / `⌄dim⌄` — pays the source-layer `rewrite_inline_extensions` scan
that the legacy renderers never run. This section records that path directly.

Both groups were captured in the same session with
`--warm-up-time 1 --measurement-time 3 --sample-size 10`. Times are Criterion's
middle (mean) estimate. The `fold_only/tree` column is the legacy-shaped
routing: no-inline fixtures fold through the plain `fold_markdown_to_document`
(no rewriter), and only `mark_dim_hr` folds span-aware. The `fold_production`
column folds **every** fixture span-aware, as production does. For a no-inline
fixture the difference between the two columns is precisely the cost the
production path adds.

| Fixture               | Inline? | `fold_only/tree` | `fold_production` | Scan overhead |
|-----------------------|---------|------------------|-------------------|---------------|
| `small_prose`         | no      | 1.20 µs          | 1.99 µs           | +0.79 µs      |
| `large_prose`         | no      | 47.49 µs         | 61.59 µs          | +14.10 µs     |
| `large_code_block`    | no      | 6.87 µs          | 34.90 µs          | +28.03 µs     |
| `large_table`         | no      | 203.97 µs        | 240.47 µs         | +36.50 µs     |
| `deeply_nested_lists` | no      | 10.15 µs         | 12.90 µs          | +2.75 µs      |
| `many_links_images`   | no      | 89.68 µs         | 137.44 µs         | +47.76 µs     |
| `image_heavy`         | no      | 88.61 µs         | 137.57 µs         | +48.96 µs     |
| `mark_dim_hr`         | yes     | 173.88 µs        | 170.94 µs         | ≈ 0 (same path) |

The no-inline production overhead is the `rewrite_inline_extensions` scan and
nothing more. `scan_delimiters` makes one linear pass; finding no `==` / `⌄`
candidate, the rewriter returns the borrowed source unchanged and **never**
reaches the protected-region pre-parse or the rewrite allocation
(`inline_extension.rs`, the `delimiters.is_empty()` fast return). The overhead
therefore tracks document size, not structure: `large_code_block` shows the
largest *relative* jump (≈5×) only because folding one 600-line code block is
otherwise trivial, so the ~28 µs linear scan of its ~30 KB source dominates;
the absolute cost is still small and is paid once per render.

The `mark_dim_hr` row is the control: it routes span-aware in **both** groups,
so the two columns coincide within run-to-run noise (≈3 µs), confirming the
cross-group comparison is sound and that the rewrite scan is the only variable
the no-inline rows isolate.

## Recorded Baselines (2026-06-02, Prose cross-target after full collapse)

`Prose` is the 132-file inline-text hot primitive. Its full collapse onto the
shared render tree (`../../2026-06-02-prose-tree/spec.md`) deleted the bespoke
`terminal` / `browser` / `to_markdown` emitters and routes every target through
the shared tree renderers instead. The prose-tree spec's "Performance" section
requires a Prose render benchmark over terminal + browser + markdown with no
material regression versus the bespoke emitters before the feature flips.

This section records that baseline. It feeds the perf-gate spec
(`../../2026-06-02-perf-gate/spec.md`): it is a **Part-2 baseline-trend** entry,
not a Part-1 bespoke comparison. There are **no Tree/Bespoke ratios** because
the bespoke emitters no longer exist — they were deleted in the migration's
step 6, so a live before/after arm cannot be measured (the pre-flip output
survives only in git history, and `prose/parity.rs` is the byte-stable parity
oracle that locked the flip's correctness). Per the prose-tree review-3
decision, the current tree-only numbers are recorded as the **defensible known
baseline** for trend tracking; the perf-gate suite's terminal-only Prose
component bench will later layer onto this.

### Capture Procedure (biscuit-terminal `rendering` bench)

This baseline lives in a different crate and bench than the darkmatter
`migration_parity` rows above — it is the `prose_render` group of
`biscuit-terminal/lib/benches/rendering.rs`.

```bash
# Run only the Prose cross-target group and save the trend baseline.
cargo bench -p biscuit-terminal --bench rendering -- prose_render \
    --warm-up-time 1 --measurement-time 3 --sample-size 10 \
    --save-baseline prose-tree-2026-06-02

# Compare a later run against it.
cargo bench -p biscuit-terminal --bench rendering -- prose_render \
    --baseline prose-tree-2026-06-02
```

### Corpus Coverage

| Corpus      | Shape                                                                                  |
|-------------|----------------------------------------------------------------------------------------|
| `small`     | One short CLI line, light styling (`<bold>`, `<red>`) — the common one-liner.           |
| `medium`    | 12 report clauses: weight + dim + color + a Markdown link each.                         |
| `tag_dense` | 16 deeply-nested span groups (every emphasis / color / underline / inverse variant, links, escaped markup) plus a trailing fenced code block — adversarial maximum-work. |

Each bench measures the full parse + render hot path: `Prose::new` builds the
`RenderNode` tree once, then the shared renderer folds it — the shape real
callers hit. Corpora are generated deterministically in-process.

### Numbers

Captured on the development host with `--warm-up-time 1 --measurement-time 3
--sample-size 10`; times are Criterion's middle estimate.

| Target          | `small`   | `medium`  | `tag_dense` |
|-----------------|-----------|-----------|-------------|
| terminal        | 10.68 µs  | 161.1 µs  | 463.5 µs    |
| browser         | 12.12 µs  | 202.9 µs  | 559.0 µs    |
| markdown        | 9.21 µs   | 156.8 µs  | 390.2 µs    |
| markdown_plus   | 9.54 µs   | 161.3 µs  | 405.2 µs    |

The `tag_dense` row carried the widest CIs (terminal `[428.8, 463.5, 507.9] µs`,
browser `[532.0, 559.0, 603.5] µs`, several high-severe outliers at this small
sample size); treat its trend band wider than the nominal ±5%. The `small` and
`medium` rows were stable.

Reading the matrix: the four targets sit within a narrow band of each other at
every corpus size — terminal and browser pay the SGR / HTML lowering on top of
the shared fold, while plain Markdown is consistently the cheapest (it degrades
color and inverse to inner text). No target is an outlier, which is the expected
shape once all four share one parse and one tree fold. Absolute costs scale with
input size and tag count, not with target, confirming the collapse did not push
any single target onto a materially slower path. With the bespoke allocation and
the separate `to_render_nodes()` projection pass both removed (spec
"Performance"), the net direction matches the spec's neutral-to-faster
expectation; this baseline is the trend anchor for future regression checks.

## How to Read the Numbers

- The terminal `TrueColor` and `fold_once_multi_target` groups already
  show large wins for the tree path. This is the architectural payoff the
  migration is built on.
- The `terminal_no_color` group is the documented exception: the legacy
  `ColorDepth::None` early return is dramatically faster than the tree
  fold + render. The spec's "Performance Expectations" pre-commits to
  this gap and to adding a tree no-color fast path before the public
  terminal cutover.
- The `browser` group has the tree path within ~2× of the legacy
  renderer for prose, and at parity (≈1.0×) for the code-heavy
  `large_code_block` fixture, where both pipelines pay the same syntect
  cost. The `mark_dim_hr` browser ratio (≈6×) is dominated by the
  span-aware fold cost (see the `fold_only` row, ≈164 µs) plus mark/dim
  inline node construction — **not** by HR SVG generation. The tree
  browser path lowers a `ThematicBreak` to a plain `<hr>` void tag with
  `data-hr-*` attributes (`renderable/src/tree/render/browser.rs`,
  `render_thematic_break`) and generates no SVG; the rich
  CSS-variable-driven `<svg>` is produced only by the legacy HTML path.
  The tree HR browser output is therefore cheaper but lower-fidelity than
  legacy — that fidelity downgrade is the deferred "HR CSS variables"
  parity gap from `entry-point-shape.md`, tracked for a cutover decision in
  `../2026-05-21-isolated-perf/spec.md`, not a perf cost to optimize away.
- The `large_code_block` terminal and HTML rows now include the wired
  `TerminalCodeRenderer` syntax-highlighting cost on the tree side, so they
  are the production-shaped numbers for the code-renderer cutover decision.
- The `fold_only` numbers measure tree-fold cost in isolation. They are
  the lower bound for any tree-target render and are stable across runs
  to within ±2%.
- The `fold_production` numbers (2026-06-02 section) are the production
  entry point's true per-document fold cost, including the
  `rewrite_inline_extensions` scan every document pays. For no-inline
  documents that scan is the *only* overhead over the plain fold; it is
  linear in source size and never triggers the protected-region pre-parse.

## Update Protocol

When new fixtures land or a bench changes shape:

1. Re-run `cargo bench -p darkmatter --bench migration_parity` end-to-end.
2. Update the tables above with the new middle-estimate numbers.
3. Save the baseline:
   `cargo bench -p darkmatter --bench migration_parity -- --save-baseline pre-cutover-YYYY-MM-DD`.
4. Reference the new baseline in any cutover PR's description so the
   reviewer can reproduce the comparison locally.

## Perf-Gate Baseline (2026-06-03, pre-cutover — PROVISIONAL)

Captured with the perf-gate suite (`2026-06-02-perf-gate/spec.md`) as Criterion
baseline `pre-cutover-2026-06-03` across `render_tree`, `render_pipeline_steps`,
`compose_pipeline`, and `migration_parity`
(`--warm-up-time 1 --measurement-time 3 --sample-size 10`).

> **Provisional — re-capture on a quiescent host before treating as authoritative.**
> This run showed load contamination: `migration/browser/large_code_block/legacy`
> read 164 ms here vs ≈16 ms in the 2026-06-02 full run (≈10× inflation). The
> Part-2 Criterion baseline (`pre-cutover-2026-06-03`) and the absolute numbers
> below are therefore indicative; the **ratios and gate direction are robust**
> (they compare legacy vs tree measured back-to-back within the same run).

### Part 1 — bespoke comparison (`migration_parity`, tree ÷ legacy)

**Terminal — PASS.** Geomean **0.122×** (tree ≈ 8× faster). Only `mark_dim_hr`
exceeds 1.0× (1.10×), within the 1.5× per-fixture ceiling.

**Browser — FAIL.** Geomean **3.80×**; **7 of 8 fixtures breach the 1.5×
ceiling**. The regression is the whole browser tree path, not one hotspot.

| Fixture | Legacy | Tree | Ratio | Ceiling (1.5×) |
|---|---|---|---|---|
| `small_prose` | 3.9 µs | 38.7 µs | 9.89× | ✗ breach |
| `large_prose` | 552 µs | 4.18 ms | 7.57× | ✗ breach |
| `large_code_block` | 164 ms* | 60.1 ms* | 0.37× | ✓ pass |
| `large_table` | 1.48 ms | 27.29 ms | 18.38× | ✗ breach (worst) |
| `deeply_nested_lists` | 24.0 µs | 84.6 µs | 3.53× | ✗ breach |
| `many_links_images` | 302 µs | 714.6 µs | 2.37× | ✗ breach |
| `mark_dim_hr` | 123.8 µs | 653.1 µs | 5.27× | ✗ breach |
| `image_heavy` | 347.6 µs | 680.7 µs | 1.96× | ✗ breach |

`*` load-contaminated; the `large_code_block` ratio in particular is unreliable.

### Part 2 — tree-only baseline trend

The tree-only suites (`component_render`, `render_pipeline_terminal/_browser`,
`darkmatter_components`, `compose_pipeline`) are saved as Criterion baseline
`pre-cutover-2026-06-03` for the >10% baseline-trend guard. Re-save on a
quiescent host alongside the Part-1 re-capture.

### Gate verdict and implication

- **Terminal cutover: clears Part 1** (geomean ≪ 1.0×, no ceiling breach).
- **Browser cutover: BLOCKED by Part 1.** The browser tree renderer is 2–18×
  slower than legacy on every fixture except `large_code_block`. Cutover
  Decision #9 framed this as a `large_table` hotspot; the data shows it is the
  **entire browser path**. Before Phase 5 (delete), the browser tree renderer
  needs broad optimization (the `2026-05-21-isolated-perf` spec's territory) or
  each breach needs a documented exception. This is the gate working as
  intended — it caught a broad regression the single-fixture framing understated.
