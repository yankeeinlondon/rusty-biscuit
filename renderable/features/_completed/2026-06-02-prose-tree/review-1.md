---
ready: false
agent: codex
model: ""
---

# Review: Prose to Shared Render Tree

This feature is not ready for production. The implementation has made real progress toward tree-only Prose rendering, but there are blocking parity gaps and documentation drift against the spec.

## Findings

### Critical: styled spans containing fenced code blocks now render as empty output

The spec requires code fences to remain part of the Prose parity corpus and explicitly calls out nested style restoration as a parity contract. The current parser recursively builds child `RenderNode`s inside a styled span at `biscuit-terminal/lib/src/components/prose/tokens.rs:413`, so a fenced code block inside `<red>...</red>` becomes a block-level `Code` node inside an inline `Span`. The tree validator rejects that shape, and `Prose::render_via_tree` converts the render error to empty output at `biscuit-terminal/lib/src/components/prose/render.rs:42`.

This is not hypothetical: the test at `biscuit-terminal/lib/src/components/prose/mod.rs:659` codifies the regression as an accepted limitation and asserts `""`. That directly conflicts with the feature spec's requirement to preserve code fences, layout, and nested style restoration before deleting the bespoke emitters.

Verification level: Level 1 currently asserts the broken behavior. Existing Level 2 code-block restoration coverage uses `<code-block ...>` on one line, not Markdown fenced code inside a styled span, so it does not cover this user-observable regression.

Suggested fix: split styled spans around block children during parsing/root normalization, or represent code blocks outside the inline span while preserving the inherited style restoration contract.

### High: Phase 3 parity oracle was overwritten by post-cutover expectations

The spec requires capturing current bespoke terminal/browser/Markdown/MarkdownPlus output before deletion, then diffing the tree output against that oracle with only approved diffs. `biscuit-terminal/lib/src/components/prose/parity.rs:1` still describes a pre-cutover bespoke oracle, but the deleted bespoke modules are already gone and the assertions now run through the tree-only public methods. For example, the `<hidden>` test at `biscuit-terminal/lib/src/components/prose/parity.rs:231` asserts the post-cutover literal-text behavior, not the old SGR 8/browser-hidden behavior described in the same file.

This removes the evidence needed to prove byte-stable parity across the migration. A passing "parity" suite can no longer distinguish preserved behavior from behavior that was changed before the oracle was captured.

Verification level: Level 1. The mismatch is in the test strategy itself, not just missing test cases.

Suggested fix: either restore pre-cutover snapshots from the old emitters or explicitly document that the original oracle was not preserved and rebuild parity confidence with before/after snapshots from a known baseline.

### High: no updated Level 2 proof for the new Prose tree-only terminal requirements

The request's rigor rules require real-terminal verification for user-observable terminal rendering such as SGR styling, OSC8 links, glyph widths, and wrapping. There are existing Level 2 CLI prose tests in `biscuit-terminal/cli/tests/level2_prose_styling.rs`, and they cover important basics such as red SGR, OSC8, rich styling, and wrapping. They do not appear updated for the new requirements introduced by this migration:

- `<inverse>` / `<reverse>` as shared `TextEmphasis::inverse` through the Prose tree path.
- `<hidden>` removal as visible literal text through a real terminal.
- styled Markdown fenced code blocks inside Prose, which currently fail at Level 1.
- Markdown fallback link escaping with styled child text after direct terminal inline rendering.

The strongest coverage for those changed user-visible behaviors is Level 1 in `biscuit-terminal/lib/src/components/prose/parity.rs` and `biscuit-terminal/lib/src/components/prose/mod.rs`. Under the review instructions, this is a high-severity verification gap.

Suggested fix: extend the Level 2 prose CLI suite with targeted WezTerm/Kitty captures for inverse, hidden removal, styled fenced code, and OSC8/fallback link escaping on the tree path.

### Medium: the spec says `ProseStyle` is deleted, but it remains as local parser state

The end-state architecture says `ProseDocument`, `ProseNode`, `ProseStyle`, and the bespoke emitters are deleted. `ProseDocument` and `ProseNode` are gone, but `ProseStyle` was moved into `biscuit-terminal/lib/src/components/prose/styles.rs:784` and is still used by `tokens.rs` as the tag-resolution style intent.

This may be an acceptable lightweight parser helper, but it is not the architecture described by the spec, and the comments still call `hidden` Prose-only at `biscuit-terminal/lib/src/components/prose/styles.rs:792` even though the spec says hidden is removed.

Suggested fix: either replace `ProseStyle` with direct `Style`/`RenderNode` construction in the parser, or update the spec/plan to explicitly allow a local tag-intent helper that is not a component-local rendering IR.

### Medium: public docs still describe the deleted ProseDocument path

The spec requires updating docs and changelog-level notes for Prose's tree-only status and `<hidden>` removal. Current docs still say Prose uses `ProseDocument`, has no `TreeRenderable`, and renders through a dedicated inline IR:

- `renderable/docs/components.md:40`
- `renderable/docs/components.md:138`
- `renderable/docs/components.md:179`
- `biscuit-terminal/docs/components/prose.md:6`
- `biscuit-terminal/docs/components/prose.md:19`
- `biscuit-terminal/docs/components/prose.md:139`

This is visible drift in public documentation and would mislead downstream users and future maintainers.

Suggested fix: update the component inventory, Prose component docs, and any skill docs that still identify Prose as a `ProseDocument` holdout.

## Verification Performed

- `cargo test -p biscuit-terminal --lib -- prose --color=never` passed: 213 passed, 0 failed.
- `cargo test -p renderable --lib -- markdown --color=never` passed: 91 passed, 0 failed.

I did not run Level 2 tests in this non-interactive review pass. The review above classifies the strongest verification level present from the implementation and existing test files.
