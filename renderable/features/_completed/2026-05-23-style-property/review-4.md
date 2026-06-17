---
ready: true
agent: codex
model: ""
---

# Review: Sub-Spec #4 `ul` / `ol` / `li` Split + Wiring

## Resolution (2026-05-23)

All findings addressed:

- **High #1 (UL left-margin math)** — terminal renderer now resolves
  `ul.left-margin` against `effective_width`, then caps the body width to
  `min(component_width, effective_width - left_margin)`. Alignment treats
  `(left_margin + body)` as a single block. Test
  `render_ul_left_margin_and_max_width` asserts body wraps at <= 40 cols and
  total line <= 44 (4 margin + 40 body). Level 2 coverage added.
- **High #2 (LI alignment body-only)** — marker is now emitted under the
  containing Ul/Ol layout first. When `style.li.alignment` shifts the body the
  body becomes a block on its own line so the marker column is preserved;
  width-only Li overrides (Left alignment) keep the body inline with the
  marker. Test `render_li_body_alignment_right` asserts marker stays at column
  0 and body is right-aligned on its own line. Level 2 coverage added.
- **High #3 (Level 2 list coverage)** — seven new Level 2 tests added in
  `darkmatter/cli/tests/level2_layout.rs` exercising
  `style.ul.left-margin`, `style.ul.max-width`, combined margin/max-width,
  `style.ol.alignment`, `style.li.alignment`, `--align-lists` broadcast, and
  `--align-ul` granular. All pass in a real WezTerm pane.
- **Medium #4 (deprecated Lists writes)** — `apply_cli_layout_flags`,
  `use_alignment_for_all`, and `with_fill_for_all` no longer write
  `PageComponent::Lists`. Fallback reads in `LayoutContext` are preserved.
  Tests asserting `Lists` remains unset for first-party broadcast paths added
  in both `layout/page.rs` and `cli/tests/cli.rs`.
- **Medium #5 (fallible list-margin builder)** — `try_with_list_left_margin`
  returns `PageRenderError::InvalidListLeftMarginComponent` for non-`Ul`
  components and is what `apply_list_style` uses (mapping to
  `StyleApplyError::InvalidListLeftMarginComponent`). Panicking
  `with_list_left_margin` is retained as a documented convenience for callers
  that know the component statically. Error-path tests added.

Verification: `cargo test -p darkmatter --lib` (3121 passed),
`cargo test -p darkmatter-cli --test cli` (227 passed),
`cargo test -p darkmatter-cli --test level2_layout -- --test-threads=1`
(33 passed, including the 7 new list cases) all green.

## Findings

### High: `ul.left-margin` is subtracted from `max-width`, shrinking the body below the spec

The terminal renderer resolves the `Ul` component width first, then subtracts `left_margin` from it before pushing the wrapper width ([darkmatter/lib/src/markdown/output/terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/output/terminal.rs:1370)). With `style.ul.left-margin: 4ch` and `style.ul.max-width: 40`, this renders a 36-cell body plus a 4-cell offset. The spec requires resolving the left margin first, then applying `ul.width` / `ul.max-width` to the remaining body area, so the body should still wrap at no more than 40 cells, with the 4-cell offset outside it.

The new test encodes the incorrect behavior: it asserts total line length `<= 40` and comments `left_margin (4) + body_width (36) = 40` ([darkmatter/lib/src/layout/page.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/page.rs:2219)). Browser CSS emits `margin-left: 4ch` plus `max-width: 40ch`, so terminal and browser behavior also diverge.

Fix: resolve the UL left margin against content width, reduce the available width for resolving the component fill, then apply `Max(40)` to the remaining body width. Update the test to assert a 4-cell marker offset and body wrapping at `<= 40` cells, not total line width `<= 40`.

Verification level present: Level 1, but asserting the wrong contract. Required: Level 1 for the width math, plus Level 2 for rendered terminal wrapping through a real terminal capture.

### High: `li.alignment` moves the marker, but the spec says it applies to the item body only

`Li` width/alignment overrides are pushed before marker emission ([darkmatter/lib/src/markdown/output/terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/output/terminal.rs:1399)), so `emit_styled_marker` receives the `Li` alignment offset and the bullet/number shifts with the body ([darkmatter/lib/src/markdown/output/terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/output/terminal.rs:1427)). That contradicts the spec’s requirement that `li.*` affects each item body after the marker prefix is emitted and preserves marker placement.

The test currently confirms the broken behavior rather than catching it: its comment says “The marker is part of the block and shifts with it” ([darkmatter/lib/src/layout/page.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/page.rs:2283)).

Fix: emit the list marker under the containing `Ul` / `Ol` layout, then push `Li` body width/alignment before body text only. Add assertions that the marker column is unchanged while the item text is right-aligned within the body.

Verification level present: Level 1, but asserting the wrong contract. Required: Level 1 for marker/body placement and Level 2 for visible terminal alignment.

### High: User-visible terminal list styling is only verified at Level 1

Sub-spec #4 has visible terminal requirements: UL left margin, UL max-width wrapping, UL left-margin plus max-width stacking, OL right alignment, LI body alignment, and CLI broadcast/granular list flags. The current added render tests are in-process unit tests using `Terminal::new_optimistic` and stripped output; there is no Level 2 test that runs `md` inside WezTerm/Kitty/tmux and captures rendered pane text.

Per the requested test-rigor model, real terminal rendering requirements for glyph placement, wrapping, widths, and alignment need Level 2. This feature should not be marked ready until the frontmatter path and CLI path have Level 2 coverage for the list cases above.

Fix: add Level 2 cases for `style.ul.left-margin`, `style.ul.max-width`, combined UL margin/max-width, `style.ol.alignment`, `style.li.alignment`, `--align-lists`, and one granular override such as `--align-ul`.

Verification level present: Level 1. Required: Level 2 for user-observable terminal rendering.

### Medium: Broadcast flags still write deprecated `PageComponent::Lists`

The spec says `--align-lists` / `--fill-lists` should stop writing `PageComponent::Lists` and instead write `Ul`, `Ol`, and `Li`. The implementation writes the three concrete variants and also writes the deprecated `Lists` fallback ([darkmatter/cli/src/output.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/cli/src/output.rs:148), [darkmatter/cli/src/output.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/cli/src/output.rs:183)). `use_alignment_for_all` and `with_fill_for_all` also insert `Lists` even though `PageComponent::ALL` intentionally excludes it ([darkmatter/lib/src/layout/page.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/page.rs:392)).

This weakens the split: new CLI code can cause deprecated list CSS to be emitted, and `build_component_css` has explicit deprecated `Lists` handling ([darkmatter/lib/src/layout/page.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/page.rs:980)). Compatibility should read `Lists` as fallback for external callers, but new first-party broadcasts should not keep writing it.

Fix: remove `Lists` writes from CLI broadcast and all-component broadcast paths. Keep fallback reads and dedicated compatibility tests that manually set `PageComponent::Lists`.

Verification level present: Level 1 checks broadcast state for concrete variants, but no assertion that `Lists` remains unset for new CLI paths. Required: Level 1 is sufficient.

### Medium: `with_list_left_margin` panics instead of returning the clear apply error required by the spec

The spec asks for the independent list-indent facility to accept only `PageComponent::Ul` and return a clear apply error for `Ol`, `Li`, or non-list components. The implementation defines `StyleApplyError::InvalidListLeftMarginComponent`, but the public builder panics instead ([darkmatter/lib/src/layout/page.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/page.rs:428)). The error variant is not used by the builder path.

Fix: expose a fallible builder or internal checked helper for `apply_list_style`, and reserve the panicking builder only if the API intentionally documents panic semantics. Otherwise remove the unused error variant.

Verification level present: Level 1 panic tests. Required: Level 1 error-path tests.

## Requirement Coverage

- Split `PageComponent::{Ul, Ol, Li}` and deprecated `Lists` fallback: Level 1 present.
- `style.{ul,ol,li}.{alignment,width,max-width}` lowering: Level 1 present.
- `style.ul.left-margin` independent channel: Level 1 present, but builder error behavior mismatches the spec.
- `ul.left-margin + max-width` stacking: Level 1 present but asserts the wrong width contract; Level 2 missing.
- `li.alignment` body-only behavior: Level 1 present but asserts the wrong marker behavior; Level 2 missing.
- CLI `--align-lists` / `--fill-lists` broadcast and granular overrides: Level 1 present; deprecated `Lists` write remains.
- Browser split selectors: Level 1 string assertions present; no computed CSS/render assertion.
- Active wiring warnings: Level 1 parser coverage present.

## Verification

- Attempted focused `cargo test -p darkmatter --lib ... --color=never` runs for the new list tests, but the commands collided on Cargo package-cache locks and continued compiling beyond the non-interactive session bound. I stopped the cargo processes with `pkill -f 'cargo test -p darkmatter'`; no completed test result is available from this review pass.

## Production Readiness

Not ready. The split and most lowering plumbing are present, but two core terminal behaviors are implemented contrary to the spec, and the user-visible list rendering requirements do not yet meet the required Level 2 verification.
