---
agent: codex
model: ""
ready: false
---

# Review 4

## Findings

### High - Public box-painting fields are still modeled but not rendered

Spec B settles the v1 rich visual model as `Border { color, weight, line_style, sides, radius }` and `Fill { color, intensity, band, inset }`, with terminal lowering responsible for border glyphs and painted fill bands. The public API exposes those fields: `Border::radius` at [renderable/src/style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/style.rs:333), `FillBand::Padded` / `FillBand::Indented` at [renderable/src/style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/style.rs:356), and `Fill::inset` at [renderable/src/style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/style.rs:375).

The terminal renderer still does not implement those semantics. `paint_text` treats `FillBand::Full` as full width and every other band as content width, so `Padded` and `Indented` collapse to the same output; it never reads `Fill::inset` ([biscuit-terminal/lib/src/render_tree/style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/style.rs:101)). `render_border` selects glyphs from weight and line style only and never reads `Border::radius` ([biscuit-terminal/lib/src/render_tree/style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/style.rs:397)).

This is user-observable: authors can serialize and attach indented fills, inset fills, or rounded borders, but the terminal output silently renders as a plain content-width fill or square-corner border. Strongest verification present is Level 1, and only for full-width fill and basic border glyphs ([biscuit-terminal/lib/src/render_tree/style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/style.rs:674), [biscuit-terminal/lib/src/render_tree/style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/style.rs:690)). The only `Fill::inset` coverage is serde round-trip, not rendering ([renderable/src/style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/style.rs:607)).

Required before production: implement terminal semantics for these fields with Level 1 tests for each shape and Level 2 capture for visible band width/inset/radius behavior, or remove/defer the fields from the v1 public rendering contract.

### High - Level 2 coverage still does not cover the visible styling surface

The Level 2 test file is explicitly scoped to the migrated `BlockQuote` path and checks the quote border plus styled inline content through real terminal capture ([biscuit-terminal/cli/tests/level2_render_tree_style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/cli/tests/level2_render_tree_style.rs:1), [biscuit-terminal/cli/tests/level2_render_tree_style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/cli/tests/level2_render_tree_style.rs:100), [biscuit-terminal/cli/tests/level2_render_tree_style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/cli/tests/level2_render_tree_style.rs:235)). That is useful, and the tmux case passed locally, but the spec's user-visible terminal styling surface is larger.

The current strongest tests for generic foreground/background/emphasis/fill/border are in-process Level 1 byte/string assertions ([biscuit-terminal/lib/src/render_tree/style.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/style.rs:513), [biscuit-terminal/lib/src/render_tree/render.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:2116)). Progress slot colors are Level 1 only ([biscuit-terminal/lib/src/components/progress.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/progress.rs:506)), and table stripe rendering/degradation is Level 1 only ([biscuit-terminal/lib/src/components/table/table.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/table/table.rs:3912), [biscuit-terminal/lib/src/components/table/table.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/table/table.rs:3996)).

Per the review rubric, visible SGR styling, fill bands, border glyph widths, table striping, and progress slot colors need Level 2 real-terminal capture. No Level 3 requirement is present for this feature because the spec does not define keyboard, mouse, paste, or IME behavior.

Required before production: keep the Level 1 tests for exact escape/degradation behavior, and add Level 2 cases for generic style application, fill/border geometry, table striping, and progress slot colors.

## Notes

The previously reviewed fixes for inherited text appearance, progress slot color degradation, and table stripe degradation are present and covered by Level 1 tests. `NodeAttrs` style storage, `Style` serde, `PerMode`, and migrated typed component style structs are also in place.

I could not use a `root` skill because no such skill was present in the advertised skill catalog. I used the required `renderable` skill and the repo-level AGENTS.md instructions.

## Verification

- `cargo test -p renderable style --lib --color=never`: passed, 25 tests.
- `cargo test -p biscuit-terminal render_tree_applies_style --lib --color=never`: passed, 2 tests.
- `cargo test -p biscuit-terminal progress_slot --lib --color=never`: passed, 2 tests.
- `cargo test -p biscuit-terminal stripe --lib --color=never`: passed, 23 tests.
- `cargo test -p biscuit-terminal-cli level2_block_quote_style_border_in_tmux --test level2_render_tree_style --color=never`: passed, 1 test.

## Readiness

Not ready for production. The main typed style pipeline is substantially implemented, but the public box-painting model still advertises unrendered terminal behavior and the Level 2 verification matrix remains below the rigor required for the visible styling surface.
