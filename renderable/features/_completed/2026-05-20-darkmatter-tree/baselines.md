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

## Update Protocol

When new fixtures land or a bench changes shape:

1. Re-run `cargo bench -p darkmatter --bench migration_parity` end-to-end.
2. Update the tables above with the new middle-estimate numbers.
3. Save the baseline:
   `cargo bench -p darkmatter --bench migration_parity -- --save-baseline pre-cutover-YYYY-MM-DD`.
4. Reference the new baseline in any cutover PR's description so the
   reviewer can reproduce the comparison locally.
