---
agent: codex
model: ""
ready: false
---

# Review 2

## Findings

### High - Ancestor block styles do not participate in inline style inheritance

Spec B D6 says `color` and `emphasis` inherit through render-tree traversal, while `background`, `border`, and `fill` do not. The current renderer only seeds inline inheritance from the paragraph node's own style ([biscuit-terminal/lib/src/render_tree/render.rs:329](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:329)). A styled ancestor block is applied later by wrapping already-rendered content in `render_styled()` ([biscuit-terminal/lib/src/render_tree/render.rs:176](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:176)), and styled `Span` closes with `SGR_RESET` plus only the local `effective` style ([biscuit-terminal/lib/src/render_tree/render.rs:624](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:624)).

That means a tree like "red `BlockQuote` -> paragraph -> green span -> tail text" renders the tail with terminal defaults, not red. The green span's reset clears the outer block's red SGR, and the inline renderer has no ancestor-block style to restore. This violates the inheritance contract and is user-visible in the migrated `BlockQuote` path, which now stores color on the block node ([biscuit-terminal/lib/src/components/block_quote.rs:419](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/block_quote.rs:419)).

Verification level present: Level 1 tests cover paragraph-local inheritance and span override restoration ([biscuit-terminal/lib/src/render_tree/render.rs:2144](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:2144)), but not ancestor block inheritance. Required: Level 1 byte/SGR test for block ancestor color plus child override restoration, and Level 2 capture for the user-visible nested styled output.

Suggested fix: thread an effective text-style stack through block and inline traversal. Apply box-painting at the styled node, but let inherited `color` / `emphasis` enter child paragraphs before inline spans are rendered.

### High - Progress slot colors are specified but not implemented

The spec requires `Progress` "slot colors / glyph styling" to move into declared styles or typed component style structs. The implementation's `ProgressStyle` only contains glyphs: `fill_char`, `empty_char`, `left_bracket`, and `right_bracket` ([biscuit-terminal/lib/src/components/progress.rs:28](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/progress.rs:28)). The render-tree hints carry the same glyph-only fields ([renderable/src/tree/attrs.rs:91](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/tree/attrs.rs:91)), and `render_progress_bar()` only repeats those glyphs without any styled filled/empty/bracket segments ([biscuit-terminal/lib/src/render_tree/render.rs:1134](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:1134)).

This leaves the "slot colors" part of the requirement unimplemented. The public docs still describe progress as having "color support" ([biscuit-terminal/lib/src/components/progress.rs:52](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/progress.rs:52)), but there is no API, style field, hint, renderer lowering, or compatibility path for fill/empty/bracket color.

Verification level present: Level 1 glyph tests and serde tests only ([biscuit-terminal/lib/tests/progress_parity.rs:245](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/tests/progress_parity.rs:245), [biscuit-terminal/lib/src/components/progress.rs:374](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/progress.rs:374)). Required: Level 1 tests for typed progress slot colors and emitted SGR, plus Level 2 capture showing the colored filled track, empty track, and brackets render through a real terminal.

Suggested fix: extend `ProgressStyle` with typed style/color slots for filled track, empty track, and brackets, project them through `ProgressHints` or a typed style hint, and render the segments through the shared style lowering path.

### High - Table striping still bypasses `Style` and capability-aware color degradation

Spec B's goals are that a component declares appearance and never hand-writes ANSI, and that a `Color` in a style degrades across truecolor, 256-color, and 16-color terminals. Table striping still carries only booleans in `TableStyle` ([biscuit-terminal/lib/src/components/table/types.rs:29](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/table/types.rs:29)) and `TableTerminalHints` ([renderable/src/tree/attrs.rs:384](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/tree/attrs.rs:384)). Both bespoke and tree renderers gate striping to `ColorDepth::TrueColor` ([biscuit-terminal/lib/src/components/table/table.rs:1340](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/table/table.rs:1340), [biscuit-terminal/lib/src/render_tree/render.rs:980](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:980)).

The actual stripe colors are hard-coded `48;2` / `38;2` escapes in component code ([biscuit-terminal/lib/src/components/table/table.rs:2076](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/table/table.rs:2076)). On 256-color and 16-color terminals, the feature is silently disabled instead of degraded, so this does not satisfy the spec's degradation contract or the "no bespoke ANSI" goal.

Verification level present: Level 1 tests assert truecolor output and no truecolor output without truecolor support. Required: Level 1 tests for Basic, Enhanced, TrueColor, and None using the same color degradation rules as `Style`, plus Level 2 capture for striped table rows in a real terminal.

Suggested fix: model stripes as typed color/style slots, route them through shared `Color` lowering, and keep the boolean builders as compatibility shims that select default stripe styles.

### High - Level 2 coverage is too narrow for user-visible style behavior

The new Level 2 test covers only the migrated `BlockQuote` left border glyph and border color through `bt quote` in WezTerm/Kitty/tmux ([biscuit-terminal/cli/tests/level2_render_tree_style.rs:65](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/cli/tests/level2_render_tree_style.rs:65)). The spec's user-visible surface is broader: foreground/background color, emphasis, fill bands, all-sides and left-only borders, table striping, progress slot styling, and inline span style inheritance.

The strongest coverage for most of those requirements remains Level 1 byte assertions in `render_tree::style` and `render_tree::render`. Under the requested rigor rubric, visible SGR colors, borders, fills, and glyph widths need Level 2 capture. No Level 3 requirement is present because this feature does not specify keyboard, mouse, paste, or IME behavior.

Suggested fix: add Level 2 cases for generic render-tree foreground/background/emphasis/fill/border, table striping, progress slot colors once implemented, and nested inline span restoration under a styled block ancestor.

## Verification

- `cargo test -p renderable style --lib --color=never`: passed, 25 tests.
- `cargo test -p biscuit-terminal --lib render_tree_inline_span --color=never`: passed, 2 tests.
- `cargo test -p biscuit-terminal progress_style --lib --color=never`: passed, 2 tests.
- `cargo test -p biscuit-terminal --test progress_parity custom_ --color=never`: passed, 3 tests.

## Readiness

Not ready for production. The first-review implementation gaps were partially addressed, but style inheritance is still incomplete for block ancestors, progress colors are missing, table striping remains outside the shared style/color pipeline, and Level 2 verification does not yet cover the user-visible style surface required by the spec.
