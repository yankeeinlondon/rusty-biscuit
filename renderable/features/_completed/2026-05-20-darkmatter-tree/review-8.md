---
ready: false
agent: codex
model: ""
---

# Review 8

## Findings

### High: DMTR-5 parity coverage is still incomplete for spec-listed behavior

The spec requires the legacy-vs-tree parity suite to cover rich code-block behavior, image width behavior, Mermaid modes, and parser-option-sensitive constructs including footnotes, superscript, and subscript (`spec.md:274`-`287`). It also requires tree-backed Browser/HTML, Terminal, Markdown, and MarkdownPlus behavior to be either equivalent to legacy behavior or explicitly classified (`spec.md:289`-`298`).

The current parity corpus does not cover several of those requirements. `FIXTURES` includes plain prose/headings, inline emphasis, links/images, lists/task lists, a simple code fence, table, blockquote, raw HTML, mark, dim, and one simple HR attribute fixture (`darkmatter/lib/tests/render_tree_parity.rs:112`-`161`). There is no parity fixture for footnotes, superscript, subscript, Mermaid off/text/image modes, code-block titles/line numbers/highlights, or image width behavior. Footnotes and super/subscript have fold-only unit tests (`darkmatter/lib/src/markdown/render_tree/fold.rs:1033`-`1115`), but those do not verify any target renderer or legacy-vs-tree semantic parity.

Verification level: strongest present coverage for footnotes/superscript/subscript is Level 1 fold-only. For the user-observable rendering requirements, the required legacy-vs-tree target parity is absent rather than merely under-leveled. Add parity fixtures and ledger entries for each missing category, including explicit accepted-divergence entries for deferred Mermaid/code-highlighting behavior, before marking the feature ready.

### Medium: the span-aware HR parser drops legacy malformed-attribute fallback behavior

The legacy `RuleProcessor::parse_attributes` documents and implements fallback to the legacy ad-hoc splitter when YAML parsing fails, explicitly to keep "malformed-but-previously-accepted inputs" working (`darkmatter/lib/src/markdown/block/rule_processor.rs:219`-`260`, `darkmatter/lib/src/markdown/block/rule_processor.rs:321`-`323`). The span-aware helper used by the tree path calls a separate `parse_attribute_block`, but that helper returns default attributes for any non-mapping parse result or parse error (`darkmatter/lib/src/markdown/block/rule_processor.rs:12`-`18`, `darkmatter/lib/src/markdown/block/rule_processor.rs:53`-`68`).

That means the legacy and tree HR paths can classify the same `--- { ... }` paragraph as a styled HR, but disagree on the recovered attributes for malformed or YAML-hostile inputs that the public renderer intentionally tolerated. The existing span-aware fold test covers only valid YAML-style HR attributes (`darkmatter/lib/src/markdown/render_tree/fold.rs:1261`-`1283`), and the parity fixture uses only `--- { style: waves }` (`darkmatter/lib/tests/render_tree_parity.rs:158`-`160`).

Verification level: Level 1 coverage exists only for the happy path. Either share the exact legacy parsing function/fallback path, or add a parity test and ledger entry documenting that malformed-but-previously-accepted HR attributes are an accepted tree-path divergence.

## Verification Matrix

| Requirement | Strongest verification found | Assessment |
|---|---:|---|
| Experimental tree entry points for HTML, Terminal, Markdown, MarkdownPlus | Level 1 unit smoke tests in `entrypoints.rs` | Adequate for internal entry point existence. |
| Mark/dim/HR source-aware fold | Level 1 exact span/fold tests | Adequate for covered fixtures. |
| Mark/dim/HR terminal rendering | Optional Level 2 WezTerm tests plus Level 1 entry-point tests | Adequate when Level 2 is enforced in CI. |
| Raw HTML escapes by default in tree HTML entry point | Level 1 entry-point tests | Adequate for HTML string output. |
| Footnotes, superscript, subscript target parity | Level 1 fold-only tests | Gap. |
| Rich code options, Mermaid modes, image width parity | No matching parity fixture found | Gap. |
| Benchmark corpus | Criterion harness and baseline note present | Adequate for this stage, with documented no-color terminal regression. |

## Notes

- The prior findings around escaped mark provenance and generated HR source ranges appear addressed by exact range tests in `span.rs` and fold-level tests in `fold.rs`.
- The required `root` skill was not available in this session's skill catalog, so this review used the provided root-level instructions and the required `renderable` skill.
- I started `cargo test -p darkmatter render_tree --color=never`; the workspace was still compiling dependencies during review, so I do not claim a passing test run here.
