---
ready: false
agent: codex
model: ""
---

# Review: Inline Span Extensions, Iteration 2

## Findings

### Medium: Production-path benchmark data is still not recorded for no-inline documents

The implementation added the right benchmark shape: `migration/fold_production` folds every fixture through `fold_markdown_spanned_with_frontmatter`, matching the tree entry point that always routes through the span-aware rewrite fold ([migration_parity.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/benches/migration_parity.rs:509), [entrypoints.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/entrypoints.rs:55)). That closes the measurement-design gap from review 1.

The recorded baseline document, however, only captures the `mark_dim_hr` rerun and explicitly says the other fixture rows are unchanged ([baselines.md](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/features/_completed/2026-05-20-darkmatter-tree/baselines.md:181)). The spec asks for `migration_parity` receipts after the cutover, and the plan's validation says the recorded numbers should include the new inline-span path ([plan.md](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/features/2026-05-26-inline-span/plan.md:372)). Without the `fold_production` rows, the no-inline requirement is still unverified in the artifact reviewers will consult: "documents without darkmatter inline" are expected to pay only the cheap scan, but the recorded numbers do not show that production-path cost.

Verification level present: benchmark harness code exists, but recorded benchmark evidence is incomplete. This is not a user-observable terminal/browser behavior gap, but it is a release-readiness gap for the performance claim.

### Low: The fold module documentation still describes mark/dim and HR attributes as deferred

The top-level `fold.rs` module docs still say `==mark==`, dim inline styles, and HR attributes are deferred to a follow-up feature and produced by the old `InlineStyleProcessor` / `RuleProcessor` path ([fold.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/fold.rs:9)). That is now stale: the same module implements `fold_markdown_spanned_with_frontmatter`, the inline source rewriter, HR attribute lowering, and the strikethrough dispatcher.

This violates the repo's comment-drift rule for behavior-changing edits. The code appears to be the source of truth; update or delete the obsolete paragraph so future readers do not believe the new functionality is intentionally absent.

## Fixed Since Review 1

The two functional blockers from review 1 are addressed. The envelope is now pipe-free (`{{!TOKEN!}}` + U+FDD0), and both unit and fold tests cover mark inside GFM table cells ([inline_extension.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/inline_extension.rs:714), [fold.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/fold.rs:1817)). The rewriter also protects inline code, fenced/indented code, raw HTML, link destinations, and image constructs before pairing delimiters ([inline_extension.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/inline_extension.rs:252), [inline_extension.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/inline_extension.rs:745)).

The terminal verification mismatch is also addressed in shape. Level 2 WezTerm tests now drive the span-aware fold through a real terminal pane for `mark` reverse-video SGR and `dim` SGR ([level2_render_tree_terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/tests/level2_render_tree_terminal.rs:122), [level2_render_tree_terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/tests/level2_render_tree_terminal.rs:439)).

## Coverage Assessment

User-observable requirements:

- `mark` folds to `Extended { token: "mark" }` and renders as `<mark>` in browser, `==...==` in Markdown, and reverse-video SGR in terminal. Strongest verification found: Level 1 renderer/fold tests plus Level 2 real-terminal SGR capture.
- `dim` folds to `Extended { token: "dim" }` and renders as opacity span, `⌄...⌄`, and dim SGR. Strongest verification found: Level 1 renderer/fold tests plus Level 2 real-terminal SGR capture.
- Protected Markdown regions keep literal delimiters. Strongest verification found: Level 1 rewriter and fold tests, which is appropriate because this is parser/fold behavior rather than terminal encoder behavior.
- Table-cell inline spans preserve GFM table shape. Strongest verification found: Level 1 fold tests, which is appropriate for parser structure.

## Verification

I attempted targeted Level 1 Cargo tests for `darkmatter`, `renderable`, and `biscuit-terminal`, but Cargo waited on existing package/build locks and then moved into a compile that exceeded the non-interactive review window. I terminated only the four Cargo test processes I started. No successful local test run is recorded in this review.

## Recommendation

Not ready for production until the performance baseline artifact records the production-path `fold_production` results for no-inline fixtures and the stale `fold.rs` module docs are corrected. I did not find a remaining functional blocker in the inline rewrite, fold dispatcher, renderer lowerings, or Level 2 terminal coverage.
