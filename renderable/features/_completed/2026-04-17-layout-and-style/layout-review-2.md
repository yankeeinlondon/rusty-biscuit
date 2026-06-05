---
agent: codex
model: ""
ready: false
---

# Review 2

## Findings

### High - The feature still has non-cosmetic render-tree-behind drift

Spec A's goal is to burn down the layout-related drift slice, and the new `just drift-report` recipe makes the intended exit condition explicit: `0 non-cosmetic render-tree-behind entries` ([renderable/justfile:157](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/justfile:157), [renderable/justfile:159](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/justfile:159)). The current implementation does not meet that bar. Running `just drift-report` reports:

- `biscuit-terminal`: 4 non-cosmetic `render-tree behind` entries, all `TwoColumn`.
- `darkmatter`: 60 non-cosmetic `render-tree behind` entries, all `YamlBlock`.
- Total: 64 non-cosmetic render-tree-behind entries.

The committed ledgers agree: `TwoColumn` at `width_40` is marked `Verdict::TreeBehind` for `Exact`, `Text`, `Indent`, and `Width` ([biscuit-terminal/lib/tests/render_comparison.rs:231](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/tests/render_comparison.rs:231)), and every `YamlBlock` matrix entry is marked `TreeBehind`, including layout-observable facets such as `Indent`, `BlankLines`, and `Width` ([darkmatter/lib/tests/render_comparison.rs:94](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/tests/render_comparison.rs:94), [darkmatter/lib/tests/render_comparison.rs:143](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/tests/render_comparison.rs:143)).

This is not just bookkeeping. The snapshot for `TwoColumn/width_40` shows the bespoke renderer wrapping columns into two rows while the tree renderer keeps both columns on one line ([biscuit-terminal/lib/tests/snapshots/layout_matrix__TwoColumn__width_40.snap:5](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/tests/snapshots/layout_matrix__TwoColumn__width_40.snap:5)). The `YamlBlock` baseline snapshot shows the tree path rendering a plain fenced code block instead of the bespoke formatted code block ([darkmatter/lib/tests/snapshots/layout_matrix__YamlBlock__baseline.snap:5](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/tests/snapshots/layout_matrix__YamlBlock__baseline.snap:5)). Some of the `YamlBlock` cause may be the deferred `CodeRenderer` hook, but it is still recorded as non-cosmetic tree-behind drift across layout scenarios.

Verification level present: Level 1. Required level: Level 1 for deterministic renderer parity, plus Level 2 for terminal-rendered width/indent/blank-line behavior before production. The current state fails the Level 1 parity exit condition.

Suggested fix: either eliminate the remaining non-cosmetic `TreeBehind` entries or explicitly narrow the Spec A production scope and move the remaining entries out of the feature's exit condition. For `TwoColumn`, fix the tree renderer's narrow-width column wrapping. For `YamlBlock`, wire the code rendering path or reclassify only after proving the remaining differences are genuinely out of Spec A.

### High - Terminal layout behavior is only verified at Level 1

The spec asserts user-visible terminal behavior for margins, alignment, max-width, wrapping, blank lines, and width saturation. The current test matrix exercises those scenarios, but it renders through `Terminal::new_optimistic` and captures strings directly in-process ([biscuit-terminal/lib/tests/layout_matrix_support/mod.rs:201](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/tests/layout_matrix_support/mod.rs:201), [darkmatter/lib/tests/layout_matrix_support/mod.rs:190](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/tests/layout_matrix_support/mod.rs:190)). The review-specific `max_width` and strictness tests are also direct renderer calls ([biscuit-terminal/lib/src/render_tree/render.rs:1755](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:1755)).

That is useful Level 1 coverage, but it does not prove the rendered output survives a real terminal emulator's width handling, blank-line behavior, SGR handling, or pane capture. The prompt's rigor rubric calls this out directly: terminal glyphs, widths, styling, and scrolling require Level 2 capture via a real terminal emulator or multiplexer. I found existing Level 1 PTY tests elsewhere, but no Level 2 `wezterm cli get-text`, `kitty @ get-text`, or `tmux capture-pane` coverage for this layout feature.

Verification level present: Level 1. Required level: Level 2 for terminal layout output because the requirements are user-observable terminal rendering behavior. This mismatch is a production-readiness gap.

Suggested fix: add env-gated or capability-gated Level 2 tests that render representative layout matrix cases inside tmux, WezTerm, or Kitty and capture pane text. At minimum cover margins, vertical blank lines, `max_width`, center/right alignment under a width cap, narrow `TwoColumn`, and `YamlBlock` code block layout.

### Medium - The darkmatter deferral is documented, but the bridge test does not prove the documented compatibility claim

Review 1 allowed the darkmatter migration to remain deferred only if the deferral was documented and tested as the accepted compatibility boundary. The documentation now states that `DarkmatterPage` builder output is identical to constructing an equivalent `renderable::layout::Layout` and converting through the bridge ([darkmatter/lib/src/layout/mod.rs:46](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/mod.rs:46)). The added tests, however, only build conversion values and call `Layout::validate()` ([darkmatter/lib/src/layout/types.rs:598](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/types.rs:598), [darkmatter/lib/src/layout/types.rs:630](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/types.rs:630)).

That does not verify the user-facing claim. It also leaves suspicious dead setup in the test: `margin` and `padding` are computed and then unused ([darkmatter/lib/src/layout/types.rs:606](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/types.rs:606)), which is exactly what the test warning reports.

Verification level present: Level 1 conversion validation only. Required level: Level 1 render comparison for the deferral boundary, because the claim is about rendered output equivalence.

Suggested fix: add a test that renders the same markdown through `DarkmatterPage` builder settings and through the documented equivalent `renderable::layout::Layout` bridge, then compares visible output for margin, fill/max-width, and alignment cases. Remove the unused setup while doing that.

## Verification

- `just drift-report` from `renderable/`: passed as a command, but reported 64 non-cosmetic render-tree-behind entries. This is the main readiness blocker.
- `cargo test -q -p renderable -p biscuit-terminal -p darkmatter`: passed. The run emitted warnings for unused imports/variables in the new tests, including `renderable/src/tree/validate.rs`, `biscuit-terminal/lib/src/render_tree/render.rs`, and `darkmatter/lib/src/layout/types.rs`.
