---
agent: codex
model: ""
ready: false
---

# Review 1

## Findings

### High - Migrated components still do not carry declared `Style` through the render tree

Spec B's v1 success criteria require `BlockQuote`, `Section`, `Table` stripe colors, and `Progress` styling to be expressed as declared `Style` or typed style structs, and then applied by the terminal tree renderer. The implementation only partially does this. `BlockQuote` stores its old text/background/left-border fields in a `Style` ([biscuit-terminal/lib/src/components/block_quote.rs:114](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/block_quote.rs:114)), but `render_tree()` explicitly omits color and border as "terminal presentation concerns" and never calls `node.attrs.set_style(...)` ([biscuit-terminal/lib/src/components/block_quote.rs:389](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/block_quote.rs:389)). The parity test bakes that in as an accepted divergence: border, color, and Prose styling are not asserted ([biscuit-terminal/lib/tests/render_tree_component_parity.rs:24](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/tests/render_tree_component_parity.rs:24)).

`Section` has the same problem in a different form. `HeadingLevel::heading_style()` exists ([biscuit-terminal/lib/src/components/section.rs:54](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/section.rs:54)), but projection builds plain heading text only ([biscuit-terminal/lib/src/components/section.rs:275](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/section.rs:275)), and both bespoke and tree rendering still use `heading_sgr()` to generate SGR directly ([biscuit-terminal/lib/src/components/section.rs:90](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/section.rs:90), [biscuit-terminal/lib/src/render_tree/render.rs:1127](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:1127)). That violates the spec's "component declares appearance; it never hand-writes ANSI" goal.

Verification level present: Level 1 semantic-only parity for `BlockQuote` and `Section`; Level 1 structural hints for table striping ([biscuit-terminal/lib/tests/table_parity.rs:176](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/tests/table_parity.rs:176)). Required level: Level 1 byte/SGR parity for the style migration, plus Level 2 capture for the visible border/color/emphasis output. This is not production-ready.

Suggested fix: attach the component-level `Style` to the projected nodes, or to typed slot hints where appropriate, and make the tree renderer consume those styles instead of reconstituting old bespoke behavior. Tighten parity tests so styled `BlockQuote`/`Section` output is compared for SGR/glyph behavior, not just visible words.

### High - Inline `Span` style and text-style inheritance are specified but not implemented

D6 says `Style` can attach to inline `Span` nodes, and only `color`/`emphasis` inherit through render-tree traversal. The data model implements `Style::inherited_from()` ([renderable/src/style.rs:465](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/style.rs:465)), but the terminal renderer never threads an inherited style through traversal. It only reads the current node's style in `render_styled()` ([biscuit-terminal/lib/src/render_tree/render.rs:175](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:175)).

Inline rendering ignores `attrs.style()` entirely. `NodeKind::Span` renders its children, then only applies class-based `mark`/`dim`/`sup`/`sub` handling ([biscuit-terminal/lib/src/render_tree/render.rs:573](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:573), [biscuit-terminal/lib/src/render_tree/render.rs:970](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:970)). A tree with a red parent paragraph and an unstyled child span will not inherit red text; a span with its own `Style` will not render it.

Verification level present: Level 1 unit tests for the pure inheritance helper in `renderable`, but no renderer test for inherited text appearance or inline span styling. Required level: Level 1 renderer tests for the byte-level SGR behavior, and Level 2 real-terminal capture for the user-visible styled inline text. This is a functional gap.

Suggested fix: add an effective-style stack to the terminal writer for text appearance, apply span styles during inline rendering, and add tests for parent color inheritance, child color override, inherited emphasis, and non-inheritance of background/border/fill.

### High - User-visible style rendering is not verified at the required level

The new render-tree style tests are in-process string assertions: `render_tree_applies_style_color_during_fold` and `render_tree_applies_style_border_during_fold` call the renderer directly and inspect output bytes ([biscuit-terminal/lib/src/render_tree/render.rs:1988](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:1988)). The lower-level style tests also call `apply_style()` directly and assert SGR/glyph strings ([biscuit-terminal/lib/src/render_tree/style.rs:585](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/style.rs:585)). These are useful Level 1 tests, but the spec's user-facing behavior is terminal color, emphasis, fill bands, and box-drawing borders.

There are Level 2 tests in this package, but they exercise `bt prose`, not render-tree `Style` or the migrated components ([biscuit-terminal/cli/tests/level2_prose_styling.rs:24](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/cli/tests/level2_prose_styling.rs:24)). Under the review rubric, requirements like "`^X` badges with specific colors" need Level 2 for real-terminal rendering; this feature similarly asserts SGR colors, border glyphs, widths, and fill bands. Current strongest coverage for these requirements is Level 1, so the verification level is wrong.

Suggested fix: add Level 2 WezTerm/Kitty/tmux capture tests for at least foreground, background, emphasis, fill band width, all-sides border, left-only border, and migrated component styling. Keep the Level 1 tests as byte-level guards.

### Medium - Implicit `Fill` tint ignores terminal color depth

The spec requires graceful degradation across 16-color, 256-color, and truecolor terminals. Explicit `Fill.color` goes through `color_sgr()`, but the implicit tint path in `fill_sgr()` returns hard-coded 24-bit background escapes regardless of `ColorDepth` ([biscuit-terminal/lib/src/render_tree/style.rs:269](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/style.rs:269)). That means a `Fill` with no explicit color still emits `48;2` truecolor escapes even on `ColorDepth::Basic`, `Minimal`, or `None`.

Verification level present: Level 1 only tests truecolor fill output ([biscuit-terminal/lib/src/render_tree/style.rs:589](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/style.rs:589)). Required level: Level 1 tests for each `ColorDepth`, plus Level 2 capture for representative fill rendering.

Suggested fix: route implicit fill tints through the same degradation path as authored colors, or suppress them when color depth is `None`.

## Verification

- `sniff repo`: ran successfully; workspace reports 59 packages.
- `cargo test -p renderable style --lib --color=never`: passed, 25 tests.
- `cargo test -p biscuit-terminal render_tree_applies_style --lib --color=never`: passed, 2 tests.

## Readiness

Not ready for production. The core data types exist and focused Level 1 tests pass, but component migration is incomplete, inline/inherited style behavior is missing from the renderer, and the user-visible terminal behavior does not have the required Level 2 verification.
