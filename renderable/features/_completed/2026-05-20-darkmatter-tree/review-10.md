---
ready: false
agent: codex
model: ""
---

# Review 10

## Findings

### High: DMTR-5 still marks production parity complete without required target coverage

The spec requires legacy-vs-tree parity fixtures for rich code blocks, image alt/title/width behavior, deterministic Mermaid modes, and parser-option-sensitive task lists, footnotes, superscript, and subscript. It also requires Browser/HTML, Terminal, Markdown, and MarkdownPlus behavior to be equivalent to legacy or explicitly classified before production readiness (`renderable/features/2026-05-20-darkmatter-tree/spec.md:274`-`298`).

The current parity corpus still covers only headings, paragraphs, inline styles, links/images, lists/task lists, a simple code fence, table, blockquote, raw HTML, mark, dim, and one HR-attribute fixture (`darkmatter/lib/tests/render_tree_parity.rs:106`-`161`). I did not find parity fixtures for footnotes, superscript, subscript, Mermaid modes, image title/width behavior, MarkdownPlus output, or code-block titles/line numbers/highlighted lines. Footnotes and native super/subscript are only asserted at fold level (`darkmatter/lib/src/markdown/render_tree/fold.rs:1033`-`1117`), and `parser-options.md` still has the parser-option parity acceptance criteria unchecked (`renderable/features/2026-05-20-darkmatter-tree/parser-options.md:103`-`108`).

Verification level: strongest coverage for footnotes/superscript/subscript is Level 1 fold-only, not target rendering parity. Rich code options, image title/width, Mermaid, and MarkdownPlus parity are absent or documented as deferred rather than represented in a target ledger. Under the review rubric, this is a high-severity readiness gap for user-observable behavior.

### High: the actual Darkmatter tree terminal path still does not use the Darkmatter code renderer

`TerminalCodeRenderer` is implemented to reproduce Darkmatter's highlighted code-block path, including header rows and syntax-highlighted output (`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:21`-`95`). However, the tree terminal entry-point adapter still constructs `TerminalRenderOptions` with `code_renderer: None` (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:182`-`186`). The Level 2 real-terminal helper and migration benchmark helper also pass `None` (`darkmatter/lib/tests/level2_render_tree_terminal.rs:135`-`140`, `darkmatter/lib/benches/migration_parity.rs:225`-`229`).

That means the user-observable tree terminal path, Level 2 capture tests, and performance baselines exercise the generic renderable fallback, not the Darkmatter code path. The simple code parity test only checks visible tokens from a plain Rust fence, so it cannot prove parity for syntax highlighting, titles, line numbers, highlighted lines, or header rendering.

Verification level: `TerminalCodeRenderer` has Level 1 unit coverage in isolation, but the entry point, Level 2 harness, parity suite, and benchmarks do not verify it. Wire the renderer into `terminal_options_from_terminal_options`, the Level 2 write path, and benchmark options, or explicitly narrow this stage so rich code-block parity is not claimed as complete.

## Verification Matrix

| Requirement | Strongest verification found | Assessment |
|---|---:|---|
| Experimental tree entry points for HTML, Terminal, Markdown, MarkdownPlus | Level 1 smoke tests | Adequate for existence only. |
| Mark/dim/HR span-aware fold and terminal styling | Level 1 target tests plus optional Level 2 WezTerm tests | Adequate if Level 2 is enforced where claimed. |
| Footnotes, superscript, subscript target behavior | Level 1 fold-only tests | Gap. |
| Rich code-block behavior through Darkmatter tree entry points | Level 1 isolated renderer tests; entry points pass `None` | Gap. |
| Image title/width behavior, Mermaid modes, MarkdownPlus parity | No matching target parity fixture found | Gap. |
| Raw HTML safe default | Level 1 adapter tests | Adequate for this stage. |

## Notes

- I used the required `renderable` skill and the `rust-testing` skill. The requested `root` skill was not present in this session's skill catalog, so I followed the repo instructions and local feature documents instead.
- The prompt paths used `renderable/root/features/...`, but the actual feature directory in this worktree is `renderable/features/2026-05-20-darkmatter-tree/`; this review is saved there.
- I attempted `cargo test -p darkmatter --test render_tree_parity --color=never`; it was still compiling after about 60 seconds, so I terminated it per the non-interactive session guidance.
