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

After this fix, the `migration/terminal_no_color` numbers below should be
re-captured before any public terminal cutover; the previously recorded
numbers reflect the **same** TrueColor tree work the colored group
measured, not a real no-color tree path.

## Code Renderer Wiring

The bench harness's `pinned_tree_terminal_options(color)` now wires
darkmatter's `TerminalCodeRenderer` into the tree terminal options (review-10
finding 2), matching the production `render_tree_terminal` entry point.
Previously the helper passed `code_renderer: None`, so the benchmarked tree
terminal path measured the render tree's plain dim-fence fallback instead of
darkmatter's syntax-highlighted code path.

The `large_code_block` tree numbers in particular must be **re-captured**
before any public terminal cutover: the previously recorded numbers reflect
the plain-fence fallback, not the syntax-highlighting cost the production path
now pays.

`pinned_browser_options()` likewise wires the `TerminalCodeRenderer` (review-11
finding 2), so the tree HTML group now pays darkmatter's syntect cost — plus
info-string title / line-number / highlight handling via the `CodeRenderer`
`meta` parameter — instead of the plain `<pre><code>` fallback. The
`large_code_block` HTML tree numbers must be re-captured for the same reason.

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

## Recorded Baselines (2026-05-20, sample subset)

Captured on the development host with
`--warm-up-time 1 --measurement-time 3 --sample-size 10`. Times are
Criterion's middle estimate; the harness emits low/middle/high CIs for
every measurement in the full report.

### `migration/terminal` (TrueColor)

| Fixture        | Legacy        | Tree         | Tree / Legacy |
|----------------|---------------|--------------|---------------|
| `small_prose`  | 6.57 ms       | 15.56 µs     | ≈ 0.0024×     |
| `mark_dim_hr`  | 6.84 ms       | 336.33 µs    | ≈ 0.049×      |

### `migration/terminal_no_color` (`ColorDepth::None`)

| Fixture        | Legacy        | Tree         | Tree / Legacy |
|----------------|---------------|--------------|---------------|
| `small_prose`  | 99.11 ns      | 16.36 µs     | ≈ 165×        |
| `mark_dim_hr`  | 3.26 µs       | 345.39 µs    | ≈ 106×        |

The no-color group is the documented spec exception: legacy's
`ColorDepth::None` early return is a fast path with no equivalent on the
tree side. The tree path remains the chosen architecture; the regression
is accepted as the spec calls out under "Performance Expectations" and
will be revisited with a tree no-color fast path before public terminal
cutover.

### `migration/browser`

| Fixture        | Legacy        | Tree         | Tree / Legacy |
|----------------|---------------|--------------|---------------|
| `small_prose`  | 5.35 µs       | 11.73 µs     | ≈ 2.2×        |
| `mark_dim_hr`  | 132.35 µs     | 180.80 µs    | ≈ 1.37×       |

### `migration/markdown`

The legacy renderer has no Markdown round-trip; only the tree path runs.

| Fixture        | Tree         |
|----------------|--------------|
| `small_prose`  | 9.36 µs      |
| `mark_dim_hr`  | (see full report) |

### `migration/fold_only`

| Fixture        | Legacy        | Tree         | Tree / Legacy |
|----------------|---------------|--------------|---------------|
| `small_prose`  | 498.47 ns     | 1.18 µs      | ≈ 2.37×       |
| `mark_dim_hr`  | 9.02 µs       | 17.60 µs     | ≈ 1.95×       |

### `migration/fold_once_multi_target`

The tree pipeline's strongest case: parse + fold once, render all three
targets (terminal + browser + markdown) from the single document. The
legacy comparator renders three targets too, each of which re-parses.

| Fixture        | Legacy        | Tree         | Tree / Legacy |
|----------------|---------------|--------------|---------------|
| `small_prose`  | 5.81 ms       | 33.26 µs     | ≈ 0.0057×     |
| `mark_dim_hr`  | 6.41 ms       | 597.94 µs    | ≈ 0.093×      |

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
  renderer; the gap closes as the corpus gets larger because the legacy
  pipeline's per-element string concatenation begins to dominate.
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
