---
ready: true
agent: codex
model: ""
---

# Review: Darkmatter Tree Rendering Migration, Iteration 2

## Resolution Notes (2026-05-20)

All five review findings have been addressed. Summary:

| Finding (severity)                              | Status   | Resolution                                                                                                                                                                                                                                                                                                                                                                |
|-------------------------------------------------|----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Entry points bypass span-aware fold (High)      | Resolved | `to_render_document` now delegates to `fold_markdown_spanned_with_frontmatter`, so every target entry point (`render_tree_html`, `render_tree_terminal`, `render_tree_markdown`) sees mark / dim / HR-attribute constructs. New entry-point smoke tests (`to_render_document_uses_span_aware_fold_for_*`, `render_tree_{terminal,html,markdown}_preserves_mark_text`).      |
| Sidecar drains nested children (High)           | Resolved | Removed the parallel sidecar from `fold_markdown_spanned_with_frontmatter`; `==mark==` / `⌄dim⌄` now push `ContainerKind::Mark` / `ContainerKind::Dim` frames directly onto the main fold stack, so nested standard containers (Emphasis, Strong, …) accumulate naturally. New test `span_aware_fold_preserves_emphasis_sibling_after_mark` pins the sibling-after-mark case. |
| HR-attribute parity is fold-only (High)         | Resolved | `render_tree_parity.rs` gains three darkmatter-inline fixtures (`mark`, `dim`, `hr_attributes`) plus matching tests that compare legacy `as_html` / `for_terminal` against the span-aware tree path on both HTML and terminal surfaces.                                                                                                                                    |
| Span-aware HR rewrites non-simple (Medium)      | Resolved | `SpannedRuleProcessor` now tracks a `paragraph_is_simple` flag and only rewrites paragraphs whose buffer is exactly one `Standard(Event::Text)` event — matching the legacy `RuleProcessor` invariant. New tests `rule_processor_does_not_rewrite_*` cover formatted and inline-code HR-attribute paragraphs.                                                              |
| Level 2 not enforced (Medium)                   | Resolved | New `just test-l2` recipe runs the Level 2 tests with `DARKMATTER_LEVEL2_REQUIRED=1`, hard-failing when WezTerm is unavailable. The recipe drives `cargo test -p darkmatter --test level2_render_tree_terminal`.                                                                                                                                                            |

## Findings

### High: Experimental entry points still bypass the span-aware fold

`to_render_document` calls `fold_markdown_with_frontmatter`, and every target entry point (`render_tree_html`, `render_tree_terminal`, `render_tree_markdown_dialect`) builds from that document (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:43`, `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:65`, `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:91`, `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:125`). That fold delegates to `fold_markdown_to_document_with_metadata`, which consumes raw `pulldown-cmark` events directly (`darkmatter/lib/src/markdown/render_tree/fold.rs:310`, `darkmatter/lib/src/markdown/render_tree/fold.rs:323`, `darkmatter/lib/src/markdown/render_tree/fold.rs:268`).

The span-aware implementation exists as `fold_markdown_spanned_with_frontmatter`, but it is not the adapter boundary used by the experimental render-tree pipeline (`darkmatter/lib/src/markdown/render_tree/fold.rs:350`). As a result, callers exercising the required tree entry points still do not get Darkmatter `==mark==`, dim spans, or HR-attribute paragraphs. This leaves DMTR-3 only partially implemented: the helper exists, but the designed tree rendering path cannot observe the feature.

Verification level: Level 1 fold tests exist for the helper, but no Level 1 target tests cover mark/dim/HR through `render_tree_html`, `render_tree_terminal`, or `render_tree_markdown`. User-observable target behavior is therefore unverified and currently broken for these constructs.

### High: Nested Markdown inside mark/dim is folded into the wrong tree shape

The span-aware fold uses a sidecar stack for mark/dim and then drains any newly appended children from the active main fold frame after each standard event (`darkmatter/lib/src/markdown/render_tree/fold.rs:398`, `darkmatter/lib/src/markdown/render_tree/fold.rs:414`, `darkmatter/lib/src/markdown/render_tree/fold.rs:432`). This breaks nested pulldown containers inside mark/dim. For the design fixture `⌄*dim and italic*⌄`, the text produced while `Emphasis` is open is drained out of the `Emphasis` frame into the dim sidecar before `End(Emphasis)` closes. The later `Emphasis` node is then emitted empty, as a sibling of the text, instead of wrapping it.

That contradicts the span-aware design's explicit fixture expectation:

```text
Span(style.dim = true)
  Emphasis
    Text "dim and italic"
```

The same failure mode applies to links, strong text, strikethrough, and other nested pulldown containers inside `==...==` or `⌄...⌄`. The current tests cover only flat mark/dim text (`darkmatter/lib/src/markdown/render_tree/fold.rs:1250`, `darkmatter/lib/src/markdown/render_tree/fold.rs:1263`) and do not include the design fixtures for dim-with-italic, mixed mark/dim, or mark in table cells.

Verification level: Level 1 exists for flat fold shape only. The strongest verification for nested inline requirements is absent, and the implementation does not satisfy the designed tree structure.

### High: HR-attribute parity is not tested through legacy-vs-tree targets

DMTR-5 requires fixtures comparing legacy and tree rendering for mark, dim, and HR attributes. The parity corpus covers standard Markdown inline styles, tables, raw HTML, and task-list text, but it has no mark, dim, or HR-attribute fixtures in `darkmatter/lib/tests/render_tree_parity.rs` (`darkmatter/lib/tests/render_tree_parity.rs:88`, `darkmatter/lib/tests/render_tree_parity.rs:407`, `darkmatter/lib/tests/render_tree_parity.rs:531`). The only HR-attribute tests I found are fold-level assertions on `fold_markdown_spanned_with_frontmatter` (`darkmatter/lib/src/markdown/render_tree/fold.rs:1287`).

This matters because HR attributes are user-observable in both legacy terminal and HTML renderers, while the current tree target entry points do not even use the spanned fold. A fold-only hint assertion cannot prove that `--- { style: waves }` renders equivalently or that accepted divergences are documented per target.

Verification level: Level 1 fold-only. Required minimum for this user-observable rendering requirement is Level 1 target parity at least, plus Level 2 for terminal styling/glyph behavior if terminal HR styles are part of the acceptance surface.

### Medium: The span-aware HR processor rewrites paragraphs that legacy would leave alone

The legacy `RuleProcessor` documents and tracks that HR-attribute replacement only applies to paragraphs containing a single text event; paragraphs with nested formatting are passed through unchanged (`darkmatter/lib/src/markdown/block/rule_processor.rs:88`, `darkmatter/lib/src/markdown/block/rule_processor.rs:126`). The span-aware `SpannedRuleProcessor` instead buffers every event, concatenates only text events, and parses that concatenated text as an HR directive (`darkmatter/lib/src/markdown/render_tree/span.rs:487`, `darkmatter/lib/src/markdown/render_tree/span.rs:495`, `darkmatter/lib/src/markdown/render_tree/span.rs:557`).

That can transform formatted input such as an HR marker plus an emphasized attribute block if the text fragments concatenate to `--- { style: waves }`, even though the legacy processor would preserve the paragraph because it is not simple. This violates the design instruction that list-item and nested-context behavior must match legacy rather than inferred desired shape.

Verification level: no Level 1 regression test covers non-simple HR-attribute paragraphs or list-item HR behavior from the span-aware design. Add explicit legacy-vs-spanned cases before relying on this processor for parity.

### Medium: Level 2 terminal coverage exists but is not enforced by default

`darkmatter/lib/tests/level2_render_tree_terminal.rs` adds real-terminal WezTerm coverage, which is the right shape for Level 2. However, the tests skip silently unless `WEZTERM_UNIX_SOCKET` is set, and they only hard-fail when `DARKMATTER_LEVEL2_REQUIRED=1` is provided (`darkmatter/lib/tests/level2_render_tree_terminal.rs:24`, `darkmatter/lib/tests/level2_render_tree_terminal.rs:44`, `darkmatter/lib/tests/level2_render_tree_terminal.rs:123`). I did not find CI or justfile wiring that sets `DARKMATTER_LEVEL2_REQUIRED=1`.

So the repository has optional Level 2 tests, but the production gate can still pass with only Level 1 verification. Under the review rubric, terminal glyphs, SGR styling, widths, and wrapping are not production-ready unless Level 2 actually runs in the gate.

Verification level: Level 2 test code present; effective default gate remains Level 1 unless the environment opts in.

## Production Readiness

Not ready for production.

The previous review's structural gaps are mostly addressed on paper, but the feature is still split between the required entry points and the span-aware implementation. The target pipeline does not use the new processor path, nested inline folding is incorrect, and mark/dim/HR behavior lacks target-level parity coverage. I would keep this experimental until the entry points use the span-aware fold, the sidecar folding model preserves nested inline containers, and Level 2 terminal coverage is enforced in the production gate.

## Verification Performed

- Read the feature spec and span-aware processor design.
- Reviewed `darkmatter::markdown::render_tree` entry points, fold, span processors, parity tests, Level 2 terminal tests, and benchmark registration.
- Ran `cargo metadata --no-deps --format-version 1` to confirm the workspace package name is `darkmatter`.
- Attempted `cargo test -p darkmatter --lib markdown::render_tree::fold::tests::span_aware_fold_emits_mark_span_with_class --color=never`; it was still compiling dependencies after roughly 30 seconds and was stopped for the non-interactive session limit, so no test result is claimed.

