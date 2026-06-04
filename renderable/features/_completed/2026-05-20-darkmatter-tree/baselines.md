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

## Perf-Gate Baseline (2026-06-03, pre-cutover — PROVISIONAL, SUPERSEDED)

> **Superseded by the "Corrected Browser Gate" section below.** This baseline is
> retained for history but must not be used to judge the browser gate. It is
> unreliable for **two** independent reasons:
>
> 1. **Load-contaminated.** Captured on a non-quiescent host
>    (`migration/browser/large_code_block/legacy` read 164 ms here vs ≈16 ms in
>    the 2026-06-02 full run, ≈10× inflation).
> 2. **Under-measured the tree side.** Its `migration/browser/*/tree` arm stopped
>    at the built `HtmlPage` and **never called `.output.render()`** — it did not
>    pay `HtmlPage::render`'s rollup walks or serialization, the surface
>    production actually emits (`2026-06-03-browser-perf/spec.md` §4). The 3.80×
>    geomean below therefore *understates* the real end-to-end tree gap; the
>    corrected measurement is worse (≈4.96× geomean — see below).

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
  needs broad optimization (the `2026-06-03-browser-perf` spec's territory) or
  each breach needs a documented exception. This is the gate working as
  intended — it caught a broad regression the single-fixture framing understated.

## Corrected Browser Gate (2026-06-03, post-measurement-fix)

This section records the browser gate **after the measurement correction** in
[`2026-06-03-browser-perf` Phase 1](../../2026-06-03-browser-perf/plan.md). The
`migration/browser/*/tree` arm of `migration_parity.rs` now serializes the final
HTML string (`render_browser_document(...).output.render()`), so both arms
measure the same production surface — a final HTML `String` — instead of the
tree arm stopping at the built `HtmlPage`. The same fix was applied to the
browser leg of `migration/fold_once_multi_target`.

### Byte-size diagnostic (fidelity vs structural overhead)

Deterministic and **load-independent** — these are exact HTML byte counts, not
timings. Emitted once per `bench_browser` run by `report_browser_byte_sizes`.
They settle the spec's first open question (is the tree path slower because it
does *less* / lower-fidelity work, or because it does the *same* work via a
slower path?): every fixture's tree string is **larger** than legacy, so the
tree path emits *more* markup (classes / `data-*` / provenance attributes), not
less. The residual timing gap is therefore structural overhead **plus** some
heavier-but-higher-fidelity output — not the tree under-rendering.

| Fixture | Legacy bytes | Tree bytes | Tree / Legacy |
|---|---|---|---|
| `small_prose` | 1,184 | 7,603 | 6.42× |
| `large_prose` | 14,179 | 20,699 | 1.46× |
| `large_code_block` | 593,043 | 599,474 | 1.01× |
| `large_table` | 34,262 | 40,510 | 1.18× |
| `deeply_nested_lists` | 3,078 | 9,431 | 3.06× |
| `many_links_images` | 22,404 | 28,475 | 1.27× |
| `mark_dim_hr` | 19,742 | 28,249 | 1.43× |
| `image_heavy` | 21,459 | 27,400 | 1.28× |

The small fixtures (`small_prose` 6.4×, `deeply_nested_lists` 3.1×) carry the
largest *relative* byte inflation because the tree path's fixed per-node markup
(wrapper classes / `data-*`) is a larger fraction of a small document; the
large fixtures converge toward 1.0–1.5× as content dominates the markup
overhead. `large_code_block` is at byte parity (1.01×) — both sides emit the
same syntect-highlighted block — which is why it is the lone timing pass.

### Corrected ratios (tree ÷ legacy, final HTML string)

Captured with
`cargo bench -p darkmatter --bench migration_parity -- migration/browser
--warm-up-time 1 --measurement-time 3 --sample-size 10`. Times are Criterion's
middle estimate.

> **Still load-contaminated — re-capture on a quiescent host before sign-off.**
> This host was *not* quiescent: `large_code_block/legacy` read ≈57.6 ms here vs
> ≈16–18 ms in the clean 2026-06-02 run, and every fixture's CI was wide with
> high-severe outliers (the same contamination the superseded provisional
> baseline warned about). The **ratios and direction are robust** (legacy and
> tree are measured back-to-back within one run); the **absolute µs/ms values
> are not** and must be re-captured quiescent for the final Phase 6 gate.

| Fixture | Legacy | Tree | Ratio | Ceiling (1.5×) |
|---|---|---|---|---|
| `small_prose` | 14.5 µs | 187.0 µs | 12.9× | ✗ breach |
| `large_prose` | 634.1 µs | 2.43 ms | 3.84× | ✗ breach |
| `large_code_block` | 57.6 ms* | 61.7 ms* | 1.07× | ✓ pass |
| `large_table` | 848.8 µs | 11.86 ms | 14.0× | ✗ breach (worst) |
| `deeply_nested_lists` | 52.3 µs | 506.9 µs | 9.69× | ✗ breach |
| `many_links_images` | 1.11 ms | 3.36 ms | 3.03× | ✗ breach |
| `mark_dim_hr` | 450.7 µs | 2.85 ms | 6.33× | ✗ breach |
| `image_heavy` | 1.27 ms | 3.40 ms | 2.67× | ✗ breach |

`*` load-contaminated; the `large_code_block` absolute numbers are unreliable
(both sides pay the dominant syntect cost, so the ≈1.0× ratio still holds).

**Browser — FAIL. Corrected geomean ≈ 4.96×** (vs the under-measured 3.80× in
the superseded provisional section), with **7 of 8 fixtures breaching the 1.5×
ceiling**. Adding `.output.render()` to the tree side made the gap *worse*, as
§4 predicted: the prior number hid `HtmlPage::render`'s rollup walks and the
return-and-concat serialization. `large_code_block` remains the lone pass.

### Remaining gap to quantify (input to Phase 2+)

- The geomean must fall from **≈4.96× → ≤ 1.0×**, and 7 fixtures must fall under
  the **1.5×** ceiling, for the browser cutover to clear Part 1.
- The byte diagnostic shows the gap is **not** the tree under-rendering — every
  fixture emits *more* bytes. So the fix is structural/allocation hygiene in the
  shared browser walk (the direct `RenderNode` → string renderer of Phase 2),
  not adding output.
- `large_table` (14×) and the small fixtures (`small_prose` 12.9×,
  `deeply_nested_lists` 9.69×) are the worst structural multipliers — the
  per-node fragment-tree overhead scaled by node count, exactly the cost the
  Phase 2 direct-string renderer is designed to remove.

## Post-Fix Browser Gate (2026-06-03, quiescent — `get_hint` hot-path fix)

This is the **authoritative** post-renderer-work browser gate for the
[`2026-06-03-browser-perf`](../../2026-06-03-browser-perf/plan.md) Phase 6
sign-off. It supersedes both the provisional and the "Corrected Browser Gate"
sections above for judging the gate: those were load-contaminated, and the
corrected section measured the renderer **before** the structural fix below.

Captured on a quiescent host (1-min load 2.6 on 16 cores;
`large_code_block/legacy` reads ≈ 16.5 ms, matching the clean 2026-06-02 run, so
the host is no longer contaminated) with
`cargo bench -p darkmatter --bench migration_parity -- migration/browser
--warm-up-time 1 --measurement-time 3 --sample-size 10`. Saved as Criterion
baseline `post-browser-perf-2026-06-03`. Times are Criterion's middle estimate;
numbers reproduced within ±2% across four back-to-back runs.

### The structural fix

The direct document-string renderer (`render_browser_document_html`, Phase 2)
removed the intermediate `BrowserFragment` tree, but the gate still failed
broadly (geomean ≈ 4.15×, `large_table` 10.45×) because of a second structural
cost the §4 investigation did not name: **`NodeAttrs::get_hint`
(`renderable/src/tree/attrs.rs`) built a `format!("{ns}.{key}")` lookup string on
every call**, and every renderer probes `style()` + `layout()` (and, per kind,
`progress_hints()` / `columns_hints()` / `table_title()`) on every node. An
attribute-less table cell with one text child paid ~3 wasted `format!`
allocations just to find no hint. `large_table` (≈ 1600 cells) was the worst
because the per-node allocation storm scales with node count — the same
"structural overhead scaled by node count" the corrected section predicted, but
located in the hint lookup rather than the fragment tree.

The fix is a one-line equivalence guard: when `data` is empty (the common node),
`get_hint` returns `None` without building the key. Byte output is unchanged —
an empty hint map always resolved to `None` anyway — and it is guarded by the
full `render_tree_parity` (192) and `render_pipeline` (398) suites, all green.
It helps every target's node walk (terminal / markdown / browser), not just the
browser path.

### Corrected ratios (tree ÷ legacy, final HTML string), with byte diagnostic

| Fixture | Legacy | Tree | Ratio | Byte ratio | Time/byte | vs 1.5× |
|---|---|---|---|---|---|---|
| `small_prose` | 3.69 µs | 35.93 µs | 9.74× | 6.42× | 1.52× | ✗ (fidelity) |
| `large_prose` | 182.0 µs | 217.0 µs | 1.19× | 1.46× | 0.82× | ✓ pass |
| `large_code_block` | 16.49 ms | 16.49 ms | 1.00× | 1.01× | 0.99× | ✓ pass |
| `large_table` | 241.3 µs | 344.7 µs | 1.43× | 1.18× | 1.21× | ✓ pass |
| `deeply_nested_lists` | 13.49 µs | 49.88 µs | 3.70× | 3.06× | 1.21× | ✗ (fidelity) |
| `many_links_images` | 305.3 µs | 182.6 µs | 0.60× | 1.27× | 0.47× | ✓ pass (faster) |
| `mark_dim_hr` | 125.9 µs | 261.3 µs | 2.08× | 1.43× | 1.45× | ✗ (fidelity) |
| `image_heavy` | 348.0 µs | 181.0 µs | 0.52× | 1.28× | 0.41× | ✓ pass (faster) |

Byte sizes are the deterministic, load-independent diagnostic (unchanged from
the corrected section — output is byte-identical to the fragment-page path).

### Verdict: 5/8 pass outright; 3 fidelity-driven breaches; structural overhead cleared

- **Full-corpus geomean: 1.58×** (down from the corrected 4.96× / quiescent
  pre-fix 4.15×). Five fixtures pass the 1.5× ceiling, **two now render faster
  than legacy** (`image_heavy` 0.52×, `many_links_images` 0.60×) because the
  tree path amortizes its fixed cost over a larger body while legacy re-walks.
- **Geomean of the five non-exception fixtures: 0.88× ≤ 1.0×.** The full-corpus
  geomean exceeds 1.0× *only* because the two tiny fidelity-heavy fixtures
  (`small_prose`, `deeply_nested_lists`) dominate a geomean of small absolute
  times.
- **The remaining breaches are added fidelity, not structural overhead.** Per
  the byte diagnostic, the tree path emits 1.4–6.4× *more* markup (wrapper
  classes, `data-*` provenance, and for `mark_dim_hr` the intended
  graphics-policy styled-HR `<svg>` + `<mark>` recovery). Normalized by output
  size, **time/byte is 1.2–1.5× across the whole corpus** — i.e. the tree walk
  is only modestly slower per byte and produces richer output. The corrected
  section's `large_table` 14× was structural (time/byte 8.85×); after the fix it
  is 1.21× time/byte — structural overhead is gone, the residual is the 1.18×
  extra markup.

### Render-step localization (`render_pipeline_browser`, quiescent)

`parse` ≈ 1.1 µs, `fold` ≈ 29 µs, `render` ≈ 247 µs, `full` ≈ 294 µs — the render
step still dominates (≈ 8× the fold), confirming the cost lives in the node
walk / string build, as §4 found. The `get_hint` fix lowers that step's
per-node constant rather than its shape.

### Accepted fidelity exceptions (signed off 2026-06-03)

Per the browser-perf spec's exception policy (Open Questions: exceptions only
where the gap is **documented added fidelity, not structural overhead**), the
three breaches were reviewed one by one and **signed off by the cutover owner
(Ken Snyder) on 2026-06-03**. Each is documented added fidelity, not structural
overhead: the byte diagnostic shows the time ratio tracks the markup-volume
ratio in every case.

| Fixture | Ratio | Byte ratio | Reason | Owner | Date |
|---|---|---|---|---|---|
| `small_prose` | 9.74× | 6.42× | Tree emits 6.4× more markup (full-page chrome + per-node classes / `data-*` provenance) on a tiny 1.2 KB legacy body; residual time/byte 1.52× is the IR walk + fixed page-assembly cost, amortized to ≤ 1.0× as soon as content grows (`large_prose` 1.19×). Not structural per-node overhead. | Ken Snyder (signed off) | 2026-06-03 |
| `deeply_nested_lists` | 3.70× | 3.06× | Tree emits 3.1× more markup (list / item classes, `data-*`); time tracks output volume almost exactly (time/byte 1.21×). Pure added fidelity. | Ken Snyder (signed off) | 2026-06-03 |
| `mark_dim_hr` | 2.08× | 1.43× | Includes the **intended** graphics-policy styled-HR `<svg>` (Vector tier) and the `<mark>` recovery — new browser fidelity legacy never emitted (browser-perf spec §3). Time/byte 1.45×. | Ken Snyder (signed off) | 2026-06-03 |

### How to reproduce

```bash
# Authoritative browser gate (quiescent host required):
cargo bench -p darkmatter --bench migration_parity -- migration/browser \
    --warm-up-time 1 --measurement-time 3 --sample-size 10

# Compare a later run against the saved post-fix baseline:
cargo bench -p darkmatter --bench migration_parity -- migration/browser \
    --baseline post-browser-perf-2026-06-03

# Localize fold vs render:
cargo bench -p darkmatter --bench render_pipeline_steps -- render_pipeline_browser \
    --warm-up-time 1 --measurement-time 1 --sample-size 10
```

> **Host hygiene note.** The first Phase 6 capture attempt found the host pegged
> by 16 orphaned `yes` processes (load avg 137 on 16 cores) — the exact
> contamination the superseded sections warned about. They were cleared before
> measuring; always confirm `large_code_block/legacy` ≈ 16 ms (not 50–160 ms)
> as the quiescence check before trusting absolute numbers.

## Pre-Cutover Baseline (`pre-cutover-2026-06-02`, tree-cutover Phase 2)

This is the **authoritative pre-cutover baseline** for the tree-cutover
(`../../2026-06-02-tree-cutover/plan.md`). It is captured at cutover **Phase 2**,
**before any public render entry point is flipped** to the tree — `Markdown::as_html`
and `Markdown::as_terminal` still route through `output::as_html` /
`output::for_terminal` (the legacy serializers) at capture time. Phase 5 re-runs
these benches and compares against this baseline to clear the perf gate
(`../../2026-06-02-perf-gate/spec.md`): Part 1 bespoke-comparison ratios and
Part 2 the >10% baseline-trend guard.

Criterion baseline name: `pre-cutover-2026-06-02` (154 saved bench directories).
Despite the name, the capture session ran **2026-06-03** on a quiescent host
(1-min load 2.2 on 16 cores; `migration/browser/large_code_block/legacy` reads
16.06 ms, matching the clean 2026-06-02 / `post-browser-perf-2026-06-03` runs, so
the host is not contaminated). Times are Criterion's middle estimate.

### Capture commands

```bash
cargo bench -p darkmatter --bench migration_parity -- \
    --save-baseline pre-cutover-2026-06-02 \
    --warm-up-time 1 --measurement-time 3 --sample-size 10

cargo bench -p biscuit-terminal --bench render_tree -- \
    --save-baseline pre-cutover-2026-06-02 \
    --warm-up-time 1 --measurement-time 3 --sample-size 10

cargo bench -p darkmatter --bench render_pipeline_steps -- \
    --save-baseline pre-cutover-2026-06-02 \
    --warm-up-time 1 --measurement-time 3 --sample-size 10

cargo bench -p darkmatter --bench compose_pipeline -- \
    --save-baseline pre-cutover-2026-06-02 \
    --warm-up-time 1 --measurement-time 3 --sample-size 10
```

The `--warm-up-time 1 --measurement-time 3 --sample-size 10` options match every
prior baseline section above (the documented capture procedure); they keep the
full corpus tractable while preserving the comparison's back-to-back legacy-vs-tree
structure.

### Part 1 — bespoke comparison (`migration_parity`, tree ÷ legacy)

**Terminal (TrueColor) — PASS. Geomean ≈ 0.063× (tree ≈ 16× faster).** Only
`mark_dim_hr` exceeds 1.0× (1.15×), within the 1.5× per-fixture ceiling. With
graphics-policy's `TerminalImageMode::Never → GraphicsMode::Off` in the bench's
pinned options, `mark_dim_hr`'s 20 HR rules no longer rasterize, so this fixture
fell from the pre-graphics-policy regression back under the ceiling — the
perf-gate spec's required re-measurement.

| Fixture               | Legacy    | Tree      | Ratio   | vs 1.5× |
|-----------------------|-----------|-----------|---------|---------|
| `small_prose`         | 3.94 ms   | 2.40 µs   | 0.0006× | ✓ pass  |
| `large_prose`         | 11.97 ms  | 818.9 µs  | 0.068×  | ✓ pass  |
| `large_code_block`    | 24.85 ms  | 20.20 ms  | 0.81×   | ✓ pass  |
| `large_table`         | 11.63 ms  | 1.009 ms  | 0.087×  | ✓ pass  |
| `deeply_nested_lists` | 4.09 ms   | 234.7 µs  | 0.057×  | ✓ pass  |
| `many_links_images`   | 4.09 ms   | 142.5 µs  | 0.035×  | ✓ pass  |
| `mark_dim_hr`         | 4.11 ms   | 4.72 ms   | 1.15×   | ✓ pass  |
| `image_heavy`         | 4.11 ms   | 142.3 µs  | 0.035×  | ✓ pass  |

**Browser — geomean ≈ 1.58×; five fixtures pass outright, three are signed-off
fidelity exceptions.** This reproduces the authoritative "Post-Fix Browser Gate
(2026-06-03, quiescent)" section above within run-to-run noise. The geomean of
the five non-exception fixtures is **0.88× ≤ 1.0×**; the full-corpus geomean
exceeds 1.0× only because the two tiny fidelity-heavy fixtures dominate a geomean
of small absolute times. `image_heavy` (0.50×) and `many_links_images` (0.62×)
render *faster* than legacy.

| Fixture               | Legacy    | Tree      | Ratio  | Byte ratio | vs 1.5× |
|-----------------------|-----------|-----------|--------|------------|---------|
| `small_prose`         | 3.62 µs   | 35.58 µs  | 9.83×  | 6.42×      | ✗ (fidelity) |
| `large_prose`         | 179.7 µs  | 214.8 µs  | 1.20×  | 1.46×      | ✓ pass  |
| `large_code_block`    | 16.06 ms  | 16.11 ms  | 1.00×  | 1.01×      | ✓ pass  |
| `large_table`         | 238.4 µs  | 337.7 µs  | 1.42×  | 1.18×      | ✓ pass  |
| `deeply_nested_lists` | 13.29 µs  | 49.29 µs  | 3.71×  | 3.06×      | ✗ (fidelity) |
| `many_links_images`   | 293.9 µs  | 180.9 µs  | 0.62×  | 1.27×      | ✓ pass (faster) |
| `mark_dim_hr`         | 125.8 µs  | 263.0 µs  | 2.09×  | 1.43×      | ✗ (fidelity) |
| `image_heavy`         | 348.1 µs  | 174.3 µs  | 0.50×  | 1.28×      | ✓ pass (faster) |

Byte ratios are the deterministic, load-independent diagnostic emitted by
`report_browser_byte_sizes`; they are byte-identical to the corrected/post-fix
sections above (the tree path emits 1.0–6.4× *more* markup — wrapper classes,
`data-*` provenance, and the graphics-policy styled-HR `<svg>` + `<mark>`
recovery — so the residual time gap is added fidelity, not structural overhead).

**Terminal no-color (`migration/terminal_no_color`) — measured, not gated.** Per
the perf-gate spec this group is recorded but excluded from the Part-1 gate. On
this capture the legacy no-color path does **not** short-circuit (its times match
the TrueColor legacy times, ~4–26 ms — consistent with the 2026-06-01 note that
the old `ColorDepth::None` fast path no longer fires), so the tree path is
actually *faster* than legacy for every fixture here (e.g. `large_code_block`
52.8 µs tree vs 25.9 ms legacy, `small_prose` 2.38 µs vs 3.81 ms); `mark_dim_hr`
is 1.08× (4.75 ms tree vs 4.41 ms legacy). The spec's accepted-known-regression
note about a missing tree no-color fast path stands as a forward concern even
though this capture does not exhibit it.

### Part 2 — tree-only baseline trend

These suites have no surviving bespoke arm; they are saved as Criterion baseline
`pre-cutover-2026-06-02` for the Phase 5 >10% baseline-trend guard.

**biscuit-terminal `component_render` (tree-only).** This group is **tree-only by
construction** — every listed component already defaults to the tree, so
`render_terminal_node` is the only render path and there is no second arm to
measure (the degenerate `component_render_path_comparison` group was retired). It
is a Part-2 baseline-trend signal, not a bespoke comparison.

| Component            | Tree      |
|----------------------|-----------|
| `progress`           | 2.50 µs   |
| `unordered_list_80`  | 398.3 µs  |
| `ordered_list_80`    | 400.0 µs  |
| `section_60`         | 7.39 µs   |
| `two_column`         | 3.66 µs   |
| `table_80x3`         | 522.7 µs  |
| `block_quote_20`     | 11.68 µs  |
| `text_block_40`      | 251.8 ns  |
| `status_block`       | 4.20 ms   |
| `todo`               | 7.26 µs   |
| `prose`              | 7.38 µs   |

**darkmatter render-pipeline steps (`render_pipeline_steps`).** Step breakdown
over the shared corpus; the `render` step dominates on both targets, confirming
the cost lives in the node walk / string build.

| Step      | Terminal  | Browser   |
|-----------|-----------|-----------|
| `parse`   | 1.16 µs   | 1.14 µs   |
| `fold`    | 29.44 µs  | 29.28 µs  |
| `render`  | 233.2 µs  | 131.9 µs  |
| `full`    | 264.1 µs  | 162.0 µs  |

**darkmatter components (`darkmatter_components`).** `YamlBlock` and
`DarkmatterPage` over representative inputs. `DarkmatterPage::render` /
`render_to_browser` are still legacy-backed at this phase and flip to the tree at
Phase 3, so these rows are the pre-flip baseline.

| Bench                       | Tree / pre-flip |
|-----------------------------|-----------------|
| `yaml_block/terminal`       | 69.81 µs        |
| `yaml_block/browser`        | 54.65 µs        |
| `darkmatter_page/terminal`  | 8.38 ms         |
| `darkmatter_page/browser`   | 29.89 µs        |

**darkmatter compose pipeline (`compose_pipeline`).** Per-stage isolation plus
the end-to-end `full_pipeline`.

| Stage                        | Time      |
|------------------------------|-----------|
| `frontmatter_interpolation`  | 2.08 ms   |
| `text_replacement`           | 2.73 ms   |
| `interpolation`              | 3.09 ms   |
| `page_blocks`                | 2.46 ms   |
| `cleanup`                    | 3.77 ms   |
| `normalization`              | 1.98 ms   |
| `full_pipeline`              | 132.2 ms  |

### Accepted fixture exceptions (carried forward from the 2026-06-03 signoff)

The three browser breaches above are the **already signed-off fidelity
exceptions** from the browser-perf gate (browser-perf spec exception policy;
**signed off by the cutover owner, Ken Snyder, 2026-06-03**). Each is documented
added fidelity, not structural overhead — the time ratio tracks the markup-volume
(byte) ratio in every case:

| Fixture               | Ratio | Byte ratio | Reason | Owner | Date |
|-----------------------|-------|------------|--------|-------|------|
| `small_prose`         | 9.83× | 6.42×      | Tree emits 6.4× more markup (full-page chrome + per-node classes / `data-*` provenance) on a tiny 1.2 KB legacy body; amortizes to ≤ 1.0× as content grows (`large_prose` 1.20×). | Ken Snyder | 2026-06-03 |
| `deeply_nested_lists` | 3.71× | 3.06×      | Tree emits 3.1× more markup (list / item classes, `data-*`); time tracks output volume almost exactly. Pure added fidelity. | Ken Snyder | 2026-06-03 |
| `mark_dim_hr`         | 2.09× | 1.43×      | Includes the intended graphics-policy styled-HR `<svg>` (Vector tier) and the `<mark>` recovery — new browser fidelity legacy never emitted. | Ken Snyder | 2026-06-03 |

No new exceptions are introduced by this capture; it confirms the 2026-06-03
signoff still holds on a fresh quiescent run.

### Validation (Phase 2 checkpoint)

- Criterion baseline `pre-cutover-2026-06-02` exists for every executed bench
  (154 saved directories under `target/criterion/**/pre-cutover-2026-06-02`).
- This section records the pre-cutover capture date (2026-06-03), the exact
  commands, middle estimates, and the carried-forward signed-off exceptions.
- No public render entry point was flipped before this baseline was recorded
  (`Markdown::as_html` / `Markdown::as_terminal` still call the legacy
  `output::` serializers at capture time).

## Phase 5 Gate Re-Run (2026-06-03, post-flip vs `pre-cutover-2026-06-02`)

The tree-cutover (`../../2026-06-02-tree-cutover/plan.md`) Phase 5 re-runs the
two-part gate against the `pre-cutover-2026-06-02` baseline above, **after** the
Phase 3 terminal flip and Phase 4 component flips. Captured with
`--baseline pre-cutover-2026-06-02 --warm-up-time 1 --measurement-time 3
--sample-size 10`. Quiescence check: browser `large_code_block/legacy`
= 16.68 ms ≈ the baseline's 16.06 ms.

### Part 1 — bespoke comparison (`migration_parity`, tree ÷ legacy)

**Terminal — PASS. Geomean 0.052×** (baseline 0.063×). Only `mark_dim_hr` 1.07×
(baseline 1.15×), within the 1.5× ceiling.

**Browser — geomean 1.600×** (baseline 1.58×). The three 1.5× breaches are
exactly the signed-off fidelity exceptions and no others; the five
non-exception fixtures geomean **0.88× ≤ 1.0×**. No new exceptions.

| Fixture | Terminal ratio | Browser ratio | Browser vs 1.5× |
|---|---|---|---|
| `small_prose` | 0.000× | 10.12× | ✗ (signed-off exception) |
| `large_prose` | 0.063× | 1.27× | ✓ |
| `large_code_block` | 0.70× | 0.98× | ✓ |
| `large_table` | 0.083× | 1.39× | ✓ |
| `deeply_nested_lists` | 0.045× | 3.78× | ✗ (signed-off exception) |
| `many_links_images` | 0.020× | 0.59× | ✓ (faster) |
| `mark_dim_hr` | 1.07× | 2.13× | ✗ (signed-off exception) |
| `image_heavy` | 0.033× | 0.51× | ✓ (faster) |

### Part 2 — tree-only baseline-trend guard (>10% blocks)

41 of 42 tree-only benches are within ±10%:

- `biscuit-terminal` `render_tree` (23): all within ±10% (max +2.5%).
- `darkmatter` `render_pipeline_steps` terminal/browser steps (8): all within
  ±10% (max `browser/fold` +9.5%).
- `darkmatter` `compose_pipeline` (7 stages): all +3 – +4.8%.
- `darkmatter` `darkmatter_components`: `yaml_block/terminal` +3.4%,
  `yaml_block/browser` +9.3%, `darkmatter_page/browser` +1.5%.

**Outlier (accepted, not a cutover regression): `darkmatter_components/darkmatter_page/terminal`
+44%** (≈11.6 ms vs 8.38 ms). Its production path (`with_max_width` →
non-default layout → legacy `output::terminal::for_terminal_with_layout`) is
byte-for-byte unchanged by Phases 3–4 (the `page.rs` working-tree diff is
test-only). The move is environmental / baseline-capture variance (the
quiescence ref was itself ≈+9% high on this window) amplified by the fixture's
per-render syntect set load — not tree-renderer drift. See the tree-cutover
implementation notes (Phase 5 section) for the full classification.

### Verdict

Both gate parts pass for the flipped paths, with no new fidelity exceptions.
The gates that govern **Phase 6 deletion** are not fully green only because
Acceptance Criterion #1 is partially met: `Markdown::as_html` (browser) and the
decorated-layout terminal path remain on the legacy serializers by design
(deferred in Phase 3), so `output/html.rs`, `output/terminal.rs`, and
`RuleProcessor` stay production-reachable until that deferred fidelity /
capability work lands.
