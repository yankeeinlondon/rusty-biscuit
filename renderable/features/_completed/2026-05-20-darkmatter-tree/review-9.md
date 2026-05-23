---
ready: false
agent: codex
model: ""
---

# Review 9

## Findings

### High: DMTR-5 still lacks required parity fixtures and ledger classifications

The spec requires parity coverage for rich code blocks, images with alt/title/width behavior, deterministic Mermaid modes, and parser-option-sensitive constructs including footnotes, superscript, and subscript (`renderable/features/2026-05-20-darkmatter-tree/spec.md:274`). It also requires each Browser/HTML, Terminal, Markdown, and MarkdownPlus behavior to be equivalent to legacy or explicitly classified (`renderable/features/2026-05-20-darkmatter-tree/spec.md:289`).

The parity corpus is still limited to a simple code fence, simple link/image fixture, task lists, table, blockquote, raw HTML, mark, dim, and one HR-attribute fixture (`darkmatter/lib/tests/render_tree_parity.rs:106`). The code parity assertion only checks visible tokens from a plain Rust fence (`darkmatter/lib/tests/render_tree_parity.rs:530`); there are no parity fixtures for code titles, line numbers, highlighted lines, image width behavior, Mermaid modes, footnotes, superscript, subscript, or MarkdownPlus target behavior. Footnotes and super/subscript only have fold-level unit tests (`darkmatter/lib/src/markdown/render_tree/fold.rs:1033`), and `parser-options.md` still lists the required parity fixtures for footnotes/superscript/subscript as unchecked (`renderable/features/2026-05-20-darkmatter-tree/parser-options.md:103`).

Verification level: strongest coverage for footnotes/superscript/subscript is Level 1 fold-only. Rich code/image/Mermaid/MarkdownPlus parity is either absent or only documented as a gap, not covered by a target parity test or ledger row. Under the review rubric this is a high-severity gap for user-observable rendering requirements, so the feature is not production-ready.

### High: the experimental terminal tree entry point never wires Darkmatter's code renderer

`darkmatter::markdown::render_tree::TerminalCodeRenderer` exists specifically to reproduce Darkmatter's syntax-highlighted code-block path and has unit tests for highlighted terminal output (`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:21`, `darkmatter/lib/src/markdown/render_tree/code_renderer.rs:142`). However, the actual tree terminal entry-point adapter still constructs `TerminalRenderOptions { code_renderer: None }` (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:182`). The Level 2 real-terminal harness and benchmark adapter also pass `code_renderer: None` (`darkmatter/lib/tests/level2_render_tree_terminal.rs:135`, `darkmatter/lib/benches/migration_parity.rs:225`).

That means the user-observable tree terminal path being exercised and benchmarked is still the renderable fallback path, not the Darkmatter code renderer. This conflicts with DMTR-5's code-block parity requirement for syntax highlighting/titles/line-number/highlight behavior and makes the code-renderer implementation mostly unused by the migration path it was built for.

Verification level: `TerminalCodeRenderer` has Level 1 unit coverage, but the actual Darkmatter tree terminal entry point, Level 2 tests, parity suite, and benches do not verify it. Wire the renderer into `terminal_options_from_terminal_options` and the test/bench adapters, or explicitly classify the code renderer as not part of this stage and remove the readiness claim for rich code-block parity.

### Medium: span-aware HR attribute parsing still diverges from legacy malformed-input fallback

The legacy `RuleProcessor::parse_attributes` treats YAML parse failures or non-mapping YAML as non-fatal and falls back to the legacy ad-hoc splitter so malformed-but-previously-accepted inputs keep working (`darkmatter/lib/src/markdown/block/rule_processor.rs:219`, `darkmatter/lib/src/markdown/block/rule_processor.rs:241`). The span-aware fold path calls the free helper `try_parse_hr_attrs`, whose `parse_attribute_block` returns default attributes for every non-mapping or parse-error case instead of using that fallback (`darkmatter/lib/src/markdown/block/rule_processor.rs:12`, `darkmatter/lib/src/markdown/block/rule_processor.rs:56`).

So legacy and tree can both recognize the same `--- { ... }` paragraph as an HR directive while disagreeing on recovered `style`, `width`, `alignment`, `weight`, or `color` fields. The current parity fixture uses only valid YAML-style attrs (`darkmatter/lib/tests/render_tree_parity.rs:157`), so this behavioral split is not pinned.

Verification level: Level 1 happy-path coverage exists; malformed accepted-input parity is absent. Share the legacy parser/fallback implementation with the span-aware helper, or add a parity fixture and accepted-divergence ledger entry for malformed HR attrs.

## Verification Matrix

| Requirement | Strongest verification found | Assessment |
|---|---:|---|
| Experimental entry points for HTML, Terminal, Markdown, MarkdownPlus | Level 1 smoke tests | Adequate for existence only. |
| Mark/dim/HR span-aware fold and terminal styling | Level 1 fold/render tests plus optional Level 2 WezTerm tests | Adequate if Level 2 is enforced in CI. |
| Footnote, superscript, subscript target behavior | Level 1 fold-only tests | Gap. |
| Rich code-block behavior through Darkmatter tree entry points | Level 1 isolated `TerminalCodeRenderer` tests, but entry points pass `None` | Gap. |
| Image width/title behavior, Mermaid modes, MarkdownPlus target parity | No matching parity fixture found | Gap. |
| Raw HTML safe default | Level 1 entry-point tests | Adequate for this stage. |

## Notes

- I used the required `renderable` skill and the `rust-testing` skill. The requested `root` skill was not present in this session's skill catalog, so I followed the root `AGENTS.md` instructions instead.
- The prompt paths used `renderable/root/features/...`, but the actual feature directory in this worktree is `renderable/features/2026-05-20-darkmatter-tree/`; this review is saved there.
- I attempted `cargo test -p darkmatter --test render_tree_parity --color=never`; it was still compiling after about 60 seconds and I killed it per the non-interactive session guidance, so I do not claim a passing test run.
