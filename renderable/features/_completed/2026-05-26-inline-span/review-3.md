---
ready: true
agent: codex
model: ""
---

# Review: Inline Span Extensions, Iteration 3

## Findings

No blocking or non-blocking findings.

The two open items from review 2 are addressed. The fold module docs now describe the source-rewrite path, provenance mapping, HR attribute lowering, and `NodeKind::Extended` dispatch instead of the deleted `InlineStyleProcessor` / `RuleProcessor` transport. The benchmark artifact now records `migration/fold_production` numbers for no-inline fixtures, so the production path's rewrite-scan cost is visible rather than inferred.

## Requirement Coverage

- Source rewrite replaces `==mark==` and `⌄dim⌄` with the pipe-free `{{!TOKEN!}}` + U+FDD0 envelope, while leaving no-extension documents borrowed unchanged. Strongest verification: Level 1 unit tests in `inline_extension`.
- Protected Markdown regions keep literal delimiters in inline code, fenced/indented code, raw HTML, link destinations, image constructs, and cross-boundary cases. Strongest verification: Level 1 unit and fold tests, which is appropriate for parser/fold behavior.
- GFM table cells containing inline mark/dim preserve table shape. Strongest verification: Level 1 rewriter and fold tests, which is appropriate for parser structure.
- Fold-side dispatch emits `Extended { token: "mark" }` / `Extended { token: "dim" }`, preserves ordinary `~~strike~~` as `Delete`, diagnoses unknown synthetic tokens, supports nested mark/dim, and maps source spans back to original byte ranges. Strongest verification: Level 1 fold tests.
- Browser lowering recovers semantic `<mark>` for `mark` and emits the dim opacity span for `dim`. Strongest verification: Level 1 renderer and entrypoint tests; this is enough for the current requirement because the observable contract is generated HTML shape, not browser layout or input behavior.
- Markdown lowering roundtrips built-in tokens back to `==children==` and `⌄children⌄`, with unknown tokens rendering children transparently. Strongest verification: Level 1 renderer tests.
- Terminal lowering renders `mark` as reverse-video SGR and `dim` as dim SGR. Strongest verification: Level 2 WezTerm capture tests, matching the required level for real-terminal SGR behavior.
- Performance receipts for the production path are recorded for no-inline and inline fixtures in `renderable/features/_completed/2026-05-20-darkmatter-tree/baselines.md`. Strongest verification: benchmark artifact, not a test tier.

## Verification

Passed:

- `cargo test -p renderable --lib extended --color=never`
- `cargo test -p biscuit-terminal --lib extended --color=never`
- `cargo test -p darkmatter --lib inline_extension --color=never`
- `cargo test -p darkmatter --lib span_aware_fold --color=never`
- `cargo test -p darkmatter --test level2_render_tree_terminal level2_tree_mark_renders_reverse_video_in_real_terminal --color=never`
- `cargo test -p darkmatter --test level2_render_tree_terminal level2_tree_dim_renders_dim_sgr_in_real_terminal --color=never`

I did not run the full workspace suite or the full Level 2 terminal file.

## Recommendation

Ready for production. I did not find a remaining functionality gap, broken implementation path, or verification-level mismatch for the inline-span requirements.
