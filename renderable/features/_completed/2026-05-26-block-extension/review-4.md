---
ready: false
agent: codex
model: ""
---

# Review: Block Extension - HR-Attribute Lift, Iteration 4

## Findings

### High - Inline rewriting still runs before HR block recognition

The implementation wires `fold_markdown_spanned_with_frontmatter` as:

1. `rewrite_inline_extensions(md.content())`
2. `Parser::new_ext(rewrite.source.as_ref(), ...)`
3. `BlockExtensionProcessor::new(parser)`

See `darkmatter/lib/src/markdown/render_tree/fold.rs:418`.

That is the reverse of the spec's required ordering. The spec says the block-extension processor runs before the inline-span dispatcher because HR-attribute syntax is an entire-paragraph construct, and matching it first removes the paragraph "before inline extension logic can split or rewrite its text" (`renderable/features/2026-05-26-block-extension/spec.md:102`).

This is not only a diagram mismatch. `rewrite_inline_extensions` scans the whole Markdown source for paired `==...==` / `⌄...⌄` delimiters and only protects code, HTML, links, images, and related verbatim ranges (`darkmatter/lib/src/markdown/render_tree/inline_extension.rs:239`). It does not protect HR attribute bodies. If a quoted HR attribute value contains delimiter-like text, for example:

```markdown
--- { kind: "==waves==" }
```

the source rewriter can rewrite the quoted scalar before pulldown-cmark parses it. The block processor then sees a paragraph split by strikethrough events, fails its exact single-`Event::Text` check (`darkmatter/lib/src/markdown/render_tree/block_extension.rs:142`), and flushes the raw paragraph instead of emitting a generated `ThematicBreak`. Under the spec's ordering, the same paragraph would be recognized first and parsed by the shared YAML attribute parser.

The current Level 1 coverage verifies normal HR attributes, earlier inline rewrites before an HR, and byte-stable snapshots for simple fixtures, but it does not cover HR attribute scalars containing delimiter-like text. Add a Level 1 regression that folds an HR attribute paragraph with `==...==` or `⌄...⌄` inside a quoted scalar and asserts it still becomes a generated `ThematicBreak` with no raw source leakage. The implementation should either run HR block recognition before source rewriting, or have the source rewriter protect HR-attribute paragraph bodies so the block processor receives the original paragraph text.

## Requirement Coverage

- HR-attribute paragraph recognition, malformed fallback, legacy `style` warning preflight, blockquote handling, list-item non-rewrite, fenced-code defense, and body-range policy are covered at Level 1 for ordinary attributes.
- The previous source-span gap is now covered at Level 1 by `span_aware_fold_hr_source_location_survives_earlier_inline_rewrite`.
- Byte-stable tree-pipeline output for `waves`, `kind_waves`, `all_attributes`, and `mark_dim_hr` is covered by Level 1 snapshots for HTML, terminal bytes, and MarkdownPlus.
- Terminal user-visible waves rendering remains covered at Level 2 by a real terminal pane capture.
- No Level 3 coverage is required; this feature has no OS keyboard input behavior.

## Verdict

Not ready for production. The main HR lift is well covered for the common fixtures, but the active pipeline still violates the spec's required block-before-inline sequencing and can produce user-visible raw HR markdown for delimiter-like text inside quoted HR attributes.

## Verification Run

- `cargo test -p darkmatter --lib block_extension --color=never` - passed, 17 tests.
- `cargo test -p darkmatter --lib span_aware_fold_hr_source_location_survives_earlier_inline_rewrite --color=never` - passed, 1 test.
- `cargo test -p darkmatter --test render_tree_hr_snapshots --color=never` - passed, 3 tests.
