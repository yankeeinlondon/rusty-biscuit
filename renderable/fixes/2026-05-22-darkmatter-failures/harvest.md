# Invariant Sweep Harvest — 2026-05-22

Output of the first `tests/render_invariants.rs` run (before any fix), sweeping
`shapes() × scenarios()` through `DarkmatterPage::render`. Two confirmed bug
classes, both **broader** than the original code-block report.

## Bug class 1 — `\x1b[K` (clear-to-edge) under active layout (I2)

- **Where:** every code shape (`code_rust`, `code_ts`, `blocks_fixture`), every
  scenario. 6 occurrences per code block (top padding row + 4 code lines + bottom
  padding row); 12 for the two-block fixture.
- **Not present** in any non-code shape — cleanly isolated to the code-block body.
- Confirms defects #1/#2: the body falls into the `\x1b[K` branch
  (`terminal.rs:1212` guard `code_width < terminal_width` is false under
  decoration). Fix: pass `Some(code_width)` whenever a layout context is active.

## Bug class 2 — constant trailing blank-line offset (I5b)

- **Where:** **every shape**, every scenario.
- **Magnitude:** trailing blank rows = `mb + 2` for heading/prose/lists/blockquote/
  table/hr/code; `mb + 1` for `image`.
- The document **body** (pre-decoration) ends with a constant 2 trailing blank
  lines (1 for image) independent of the configured bottom margin; row decoration
  then adds `mb` on top, producing the `mb + 2` total.
- This generalizes defect #3 beyond the code block's hardcoded `"\n\n"`: the
  offset is universal (document tail), so the fix belongs in the page render path
  (normalize/trim the inner body's trailing blanks before `apply_row_decoration`),
  not only in the code-block separator.

## Non-findings (correct behavior, confirmed)

- **Leading margins (I5b):** leading blank rows == `mt` for every shape/scenario —
  top margin is correct.
- **Interior rhythm (I5):** zero violations — no interior run of ≥2 blank lines.
- **Containment (I1):** zero violations — no line exceeds terminal width (note:
  `\x1b[K` paints invisibly, so I1 alone never catches the code bug; I2 is the
  decisive check).

## Resolution (Stage 3)

All harvested bug classes are fixed and the invariant matrix is green.

- **`\x1b[K` (bug class 1):** `terminal.rs` code-fence guard now passes
  `Some(code_width)` whenever a layout context is active (never clears to the
  physical edge under decoration). I2 green across all shapes/scenarios.
- **Trailing offset (bug class 2):** `DarkmatterPage::render` now calls
  `normalize_body_rhythm` (collapse ≥2 blank runs to one, strip trailing blanks)
  before `apply_row_decoration`, on the decorated path only. I5/I5b green;
  trailing blanks now equal `mb` exactly for every shape.
- **Theme contrast (#0):** code highlighters resolve their *variant* against
  `options.color_mode.inverted()` (terminal + render-tree + YamlBlock terminal
  paths). Header-pill text color and highlight-line background math key off the
  **resolved** theme background via `mode_for_background`, so single-variant
  themes (e.g. dracula) keep readable chrome (this caught a self-inflicted
  white→black-on-dark regression during implementation). I7 green; the
  single-variant no-op is pinned by `single_variant_theme_ignores_mode`.
- **Pre-existing test corrected:** `yaml_block::tests::test_dark_and_light_render_differ`
  encoded the same chrome bug (header text flipping with terminal mode on a
  fixed-background theme); it now pins a paired theme (`CODE_THEME=github`) so
  the dark/light assertion is meaningful.
- **Scoped out:** browser/HTML code contrast inversion was implemented then
  **reverted** — HTML has no live light/dark detection (its `color_mode` is
  caller-set) and inverting exposed an unrelated invisible-border chrome issue.
  Tracked as defect D for a separate decision.

Full `cargo test -p darkmatter` is green (incl. `render_comparison` parity and
re-blessed `layout_snapshots`). Note: a piped (non-TTY) `md` emits plain output,
so colored-path verification is via the invariant tests + reviewed snapshots.

## Still to cover (not structurally detectable; needs targeted invariants)

- **Defect #0 (theme contrast/inversion):** requires I7 (code surfaces resolve
  against the inverted terminal mode; non-code follow the terminal mode). Add as a
  targeted test against theme background colors.
- **I2/I3 background-rectangle geometry:** once `\x1b[K` is removed, add a check
  that code-body lines pad to exactly `effective_width` and share the header
  pill's right boundary.
