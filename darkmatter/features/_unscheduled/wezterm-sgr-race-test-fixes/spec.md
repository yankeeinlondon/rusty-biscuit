# Re-anchor Level 2 SGR Assertions on Visible Color

## Context

### The performance problem

Darkmatter's Level 2 test binaries (`darkmatter-cli::level2_layout`, 48 tests; `darkmatter-cli::level2_errors`, 3 tests) consistently land in the "slow" tier — most tests take 7–11 seconds each. The slowness is dominated by harness overhead (`wezterm cli` subprocess invocations, hard sleeps for terminal settling, polling intervals), not by the `md` binary itself.

Conservative cuts to those timings could drop most tests under the 5 s SLOW threshold and shave 30–60 s off the full Level 2 suite.

### Why we cannot just lower the timings

A May 2026 attempt reduced `settle()` (the post-keystroke sleep in `biscuit-test-harness`) from 200 ms to 50 ms. Every Level 2 test passed in isolation. Under the full suite, exactly one test failed deterministically:

```
panicked at darkmatter/cli/tests/level2_layout.rs:1785:5:
no foreground reset may appear between item bodies — both must inherit ul.color.
between="listbodyalpha\x1b[39m\r\n\x1b[38:2::251:44:54m- "
```

The test is `level2_ul_color_inherits_into_li_body`. Its fixture defines a two-item list with `style.ul.color: red-500` (Tailwind red, `rgb(251, 44, 54)`). The intent: verify both list items inherit the ul color. The assertion: after the SGR opens red and the first item body appears, **no `\x1b[39m` (foreground default) or `\x1b[0m` (full reset) may appear between** the two item bodies — that would prove the second item is *not* inheriting.

When run with the historical 200 ms `settle`, the WezTerm pane has time to finish the previous prompt's redraw before `md` starts writing. WezTerm's `get-text --escapes` then walks the cell grid and emits SGR transitions only when cells differ — and both list bodies share the same cells, so no `\x1b[39m` appears between them.

When `settle` is dropped to 50 ms, WezTerm's cell grid is still mid-redraw from the previous command's prompt when `md` begins emitting list rows. Result: the cell grid records the rows with a transient default-fg attribute between item 1's red span and item 2's red span. The next `get-text` capture serializes that transition as a spurious `\x1b[39m` even though the *user-visible* color is identical on both rows.

The same race breaks under `just test-l2` (which shares one broker pane across both `level2_layout` and `level2_errors`) when the harness timings tighten in *any* direction. Other tests likely have the same fragility but were not stressed enough by the May 2026 attempt to surface it.

### The deeper problem

`level2_ul_color_inherits_into_li_body` (and likely others — see Audit Checklist below) asserts on the *SGR transition shape* in the captured byte stream. But the WezTerm capture is **not the byte stream `md` emitted** — it is a re-serialization of the cell grid. WezTerm reserves the right to:

- Collapse same-attribute cells into one SGR span across rows.
- Emit a transition mid-row when adjacent cells differ.
- Use semicolon (`\x1b[38;2;R;G;Bm`) or ITU colon (`\x1b[38:2::R:G:Bm`) form depending on terminfo and version.
- Replace `\x1b[0m` with `\x1b[39m\x1b[49m`, or elide it entirely if the next cell shares attributes.

The file-level docstring at `darkmatter/cli/tests/level2_layout.rs:23-40` already calls this out:

> Contiguous same-attribute cells collapse into a single SGR span; the leading SGR may appear on a previous row and not re-appear on the next … per-line byte equality across two captures is unreliable.

The current assertion in `level2_ul_color_inherits_into_li_body` violates that guidance — it asserts on byte-level transition shape *between* lines.

The semantically correct claim is: **on the cell where `listbodybeta` starts, the foreground color is `(251, 44, 54)`** — not "no reset byte appears between bytes A and B in the capture stream."

### How this unblocks performance work

The harness-timing reductions need to land safely. Either:

- Spec A (Stable Broker Pane Lifetime) — fix the `just test-l2` flakiness first so we have a stable baseline to measure speedups against. Necessary, but does not address the SGR-race-shape fragility — a future timing tweak could break a different test the same way.

- This spec — rewrite SGR-shape assertions to assert on visible color of specific cells. Once a test's correctness no longer depends on WezTerm's serialization choices, harness timings become free to tune.

The two specs are complementary, not alternatives. This one is the longer-term hardening that makes the test suite robust to *future* timing changes.

---

## Goals

1. No Level 2 test asserts on SGR transition shape between cells. Every color check resolves to: "what is the truecolor foreground or background of cell `(row, col)`?"
2. A helper API in `biscuit-test-harness` exposes the cell grid for direct inspection.
3. Reducing `settle()` to 50 ms (or any value down to ~10 ms) does not break any Level 2 darkmatter test.
4. The audit catches every SGR-shape-dependent assertion in the existing Level 2 corpus, not just `ul_color`.

## Non-Goals

- Replacing the `frame.raw` / `frame.plain` interface where it is used correctly (substring presence checks, sentinel detection, OSC 8 hyperlink verification).
- Adding cell-grid inspection to Level 1 / Level 3 / browser tests.
- Asserting on visual rendering at the pixel level (font glyphs, image protocols). This spec is about color attributes per cell, not pixels.

## Background: how WezTerm exposes the cell grid

`wezterm cli get-text --escapes` re-serializes the cell grid as text + SGR. There is no `--json-cells` or `--per-cell-color` variant today. Three approaches are possible:

1. **Parse the SGR stream ourselves**, building an in-memory cell grid keyed by `(row, col) → CellAttrs { fg, bg, bold, italic, underline, … }`. This is a small ANSI-state-machine in `biscuit-test-harness` — `strip_ansi` already half of it; extending it to track attributes per cell is incremental.
2. **Use a third-party crate** like `vte` (from alacritty) or `termwiz` (from WezTerm itself) for the parsing.
3. **Issue a `wezterm cli get-cell` style query if it exists** — needs verification against the WezTerm CLI surface; likely not available.

`termwiz` (a WezTerm subcrate published to crates.io) is the most authoritative choice — it is literally the parser WezTerm itself uses. `vte` is the lighter dependency. Either is acceptable; the prototype phase decides.

## Proposed Design

### 1. New `CellGrid` type in `biscuit-test-harness`

```rust
pub struct CellGrid {
    rows: Vec<Vec<Cell>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl CellGrid {
    pub fn from_escapes(raw: &str) -> Self { /* feed bytes through vte/termwiz parser */ }
    pub fn find(&self, needle: &str) -> Option<(usize, usize)> { /* (row, col) of first match */ }
    pub fn cell(&self, row: usize, col: usize) -> Option<Cell> { /* … */ }
    pub fn row(&self, row: usize) -> Option<&[Cell]> { /* … */ }
}
```

`CapturedFrame` gains a third field:

```rust
pub struct CapturedFrame {
    pub raw: String,
    pub plain: String,
    pub cells: CellGrid,  // new
}
```

### 2. Rewrite `level2_ul_color_inherits_into_li_body`

Before:

```rust
let red_open_at = frame.raw.find(red_semi).or_else(|| frame.raw.find(red_colon));
let alpha_at = frame.raw.find("listbodyalpha");
let beta_at = frame.raw.find("listbodybeta");
let between = &frame.raw[alpha_at..beta_at];
assert!(!between.contains("\x1b[39m") && !between.contains("\x1b[0m"));
```

After:

```rust
let red = Color::Rgb(251, 44, 54);
let (alpha_row, alpha_col) = frame.cells.find("listbodyalpha").expect("alpha row");
let (beta_row,  beta_col)  = frame.cells.find("listbodybeta").expect("beta row");

assert_eq!(
    frame.cells.cell(alpha_row, alpha_col).map(|c| c.fg),
    Some(red),
    "first list body must render in ul.color"
);
assert_eq!(
    frame.cells.cell(beta_row, beta_col).map(|c| c.fg),
    Some(red),
    "second list body must inherit ul.color"
);
```

This says exactly what the test should mean. It is immune to WezTerm collapsing or splitting SGR spans because we read the *resolved* cell attribute, not the serialization.

### 3. Audit checklist (tests to migrate)

Every Level 2 test that uses one of these byte-substring patterns against `frame.raw` is suspect:

| Pattern | Why it is fragile |
|---|---|
| `frame.raw.contains("\x1b[39m")` (or `\x1b[0m`) | Tests for "no reset" or "reset present" — depends on serialization, not on the cell attribute. |
| `frame.raw.contains("\x1b[38;2;R;G;Bm")` | Same color may appear as `\x1b[38:2::R:G:Bm` (ITU colon form) or be elided when adjacent to same-color cells. |
| `frame.raw.contains("\x1b[48;2;R;G;Bm")` | Same as above for backgrounds. |
| Slicing `&frame.raw[A..B]` between two text substrings to assert on bytes in between | Two cells may share an SGR span that extends outside `[A..B]`; the SGR transition byte may appear *before* `A`. |

Initial sweep of `darkmatter/cli/tests/level2_layout.rs` finds at least:

- `level2_ul_color_inherits_into_li_body` (confirmed fragile)
- `level2_hyperlink_color_applies_inside_table` (`frame.raw.contains(red_semi) || frame.raw.contains(red_colon)`)
- `level2_page_bg_pronounced_emits_bg_sgr` (`frame.raw.contains("\x1b[0m") || frame.raw.contains("\x1b[m")`)
- `level2_code_block_inverts_to_dark_in_light_terminal` (uses `max_bg_luma_on_line` which scans for `48;2;` patterns — already cell-grid-ish but parses ad-hoc, would benefit from the shared parser)
- Likely 5–10 more; full audit is part of this spec's execution.

### 4. Backwards compatibility

`CapturedFrame::raw` and `.plain` stay. Tests that legitimately want the raw byte stream (sentinel detection, OSC 8 hyperlink presence) continue to use them. Only color/attribute assertions migrate.

The cell-grid build is cheap (linear in the byte stream) and runs once per capture. No measurable test runtime impact expected.

### 5. ITU colon vs semicolon form

The cell-grid parser must handle both `\x1b[38;2;R;G;Bm` (legacy) and `\x1b[38:2::R:G:Bm` (ITU T.416). `vte` handles both natively; `termwiz` does too. Both forms collapse to `Color::Rgb(R, G, B)` and tests then become form-agnostic for free.

## Acceptance Criteria

1. Every `frame.raw.contains("\x1b[...")` assertion in Level 2 darkmatter tests is replaced with a `frame.cells.cell(row, col).fg/bg/...` assertion *or* documented as a legitimate non-color check.
2. Reducing `settle()` to 50 ms produces no Level 2 darkmatter test failures across 3 consecutive `just test-l2` runs.
3. Reducing `run_with_timeout`'s inter-poll sleep from 50 ms to 10 ms produces no Level 2 darkmatter test failures across 3 consecutive runs.
4. The cell-grid parser has its own Level 1 unit tests with known SGR-collapse edge cases (same-color across newline, ITU colon form, `\x1b[0m` elision, RGB-then-default-then-RGB on same row).
5. The migration is committed alongside an update to `darkmatter/cli/tests/level2_layout.rs:23-40`'s docstring, replacing the "byte equality is unreliable, use semantic checks" guidance with "use `frame.cells` for color/attribute assertions; `frame.raw` is only for sentinel detection and OSC 8."

## Risks

- **Parser choice lock-in**: `termwiz` is heavier than `vte` but is the same parser WezTerm uses. `vte` is more widely battle-tested in the Rust ecosystem (Alacritty, Zellij). Either is acceptable; commit to one in the prototype phase and document why.
- **Wider cell-grid usage drift**: once `frame.cells` exists, tests in `biscuit-terminal/cli/tests/level2_layout.rs` and `biscuit-tui/cli/tests/real_terminal_render.rs` will also want it. Decide upfront whether to migrate those in the same spec or scope this to darkmatter-only.
- **Subtle parser bugs**: if the cell-grid parser disagrees with WezTerm's own renderer on edge cases (e.g. wide glyphs, combining characters), tests could pass while WezTerm renders something different. Mitigation: pick `termwiz` and trust it; or write enough Level 1 unit tests to catch divergence against canned WezTerm captures.
- **Inverse / italics / underline semantics**: these don't have a "default" sentinel value the way `Color::Default` does. The `Cell` shape needs to commit to "bool" vs "Option<bool>" semantics for those attributes. Lean toward `bool` and document that off == default.

## Estimated Effort

- Parser crate evaluation + Level 1 unit tests: 1 day
- `CellGrid` API + `CapturedFrame::cells` integration: 1 day
- Audit Level 2 darkmatter tests + migrate ~10–15 assertions: 2 days
- Soak tests (3× `just test-l2` runs with `settle = 50ms`): 0.5 day
- Documentation updates (file docstrings, skill `wezterm-harness-pitfalls.md`): 0.5 day
- **Total: ~5 days**

## Sequencing

Either ordering is valid:

- **Spec B first** (this spec): tests become robust, then the harness-timing reductions land safely whether or not the broker is fixed.
- **Spec A first** (Stable Broker Pane): test runs become deterministic, then tighter timings can be validated, then the SGR-race hardening lands as defense in depth.

Recommendation: **Spec A first**. A flaky `just test-l2` masks signal during the Spec B migration too — every failed run would require diagnosing "is this the broker flake or my new cell-grid assertion?". Lock down the harness lifetime first, then make the tests robust to timing.

## Follow-Up Work Enabled

Once color assertions are cell-grid-based, the following timing reductions can land without risk to SGR-shape-dependent tests:

1. `settle()` 200 ms → 50 ms.
2. Fold `clear` into the wrapped `md` command (single round-trip per test).
3. `run_with_timeout` poll 50 ms → 10 ms.
4. Post-sentinel sleep 250 ms → 100 ms.

Combined target: most Level 2 tests under 5 s (drop out of the SLOW tier), total `just test-l2` runtime ≤ 25 s for `darkmatter-cli`'s 51 tests.
