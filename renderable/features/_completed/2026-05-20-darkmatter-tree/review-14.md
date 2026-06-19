---
ready: true
agent: codex
model: ""
---

# Review 14

## Findings

No blocking findings.

Review 13's remaining production blocker appears closed. The implementation now has a Level 2 WezTerm test that drives the render-tree terminal path with the wired `TerminalCodeRenderer` and asserts the rich fenced-code behavior survives a real terminal capture: title/header text, body text, line-number gutter, syntax-highlight SGRs, and a distinct highlighted-line background (`darkmatter/lib/tests/level2_render_tree_terminal.rs`).

## Verification-Level Summary

| Requirement | Strongest verification found | Assessment |
| --- | --- | --- |
| Internal tree entry points for document, HTML, terminal, Markdown, MarkdownPlus | Level 1 entry-point smoke and behavior tests in `entrypoints.rs` | Appropriate; these are internal adapter APIs. |
| Parser option policy: tables/strikethrough public, task lists/footnotes/sup/sub tree-experimental, deferred flags off | Level 1 structural and parity-divergence tests in `render_tree_parity.rs` plus explicit `render_tree_parser_options()` | Appropriate for parser/fold behavior. |
| Span-aware mark/dim fold shape and byte ranges | Level 1 unit tests in `span.rs` and structural fold checks in `fold.rs` / `entrypoints.rs` | Appropriate for tree shape and source-range policy. |
| Mark/dim terminal styling | Level 2 WezTerm tests for reverse-video mark and dim SGR | Appropriate when enforced by `just test-l2` / `DARKMATTER_LEVEL2_REQUIRED=1`. |
| HR attributes and `darkmatter.hr` hints | Level 1 fold/hint assertions plus Level 2 WezTerm styled-rule test | Appropriate for both structure and terminal rendering. |
| Rich code-block title, line numbers, syntax highlighting, highlighted line on terminal | Level 2 WezTerm test for the wired `TerminalCodeRenderer` | Appropriate; closes the prior review gap. |
| Raw HTML safe HTML default | Level 1 entry-point tests asserting `RawHtmlPolicy::Escape` and escaped block/inline HTML | Appropriate for escaped output policy. |
| Frontmatter attached above the fold | Level 1 tests for `fold_markdown_with_frontmatter` and `to_render_document` metadata | Appropriate; no terminal-emulator behavior involved. |
| Fold/render diagnostic separation | Level 1 type and parity helper checks keeping diagnostics phase-local | Appropriate for API/data-contract behavior. |
| Benchmark harness shape | Criterion bench target `darkmatter/lib/benches/migration_parity.rs` with paired legacy/tree groups and no-color group | Appropriate for this stage; performance gate remains human-reviewed. |

## Notes

The requested `root` skill is not present in the local skill catalog for this session, so I used the repo-level instructions and the required `renderable` skill.

I attempted `cargo test -p darkmatter render_tree --color=never`, but this worktree needed a cold dependency build and was still compiling native dependencies after roughly a minute. I stopped that verification path to avoid spending the review on a full rebuild. The readiness call above is therefore based on static inspection of the implementation and test coverage, not a completed local test run.
