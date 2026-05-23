---
ready: false
agent: codex
model: ""
---

# Review: Darkmatter Tree Rendering Migration, Iteration 4

## Findings

### High: mixed mark/dim nesting is specified but impossible in the span processor

The span-aware design requires `==highlighted and ⌄dim within mark⌄==` to fold as `Span.mark` containing a nested dim `Span` (`span-aware-processor-design.md:432`). The implementation still models only one active Darkmatter delimiter at a time: `open: Option<OpenSpan>` is documented as mutually exclusive (`darkmatter/lib/src/markdown/render_tree/span.rs:166`), and a dim delimiter seen while mark is open is emitted as literal text (`darkmatter/lib/src/markdown/render_tree/span.rs:362`). That means the designed mixed fixture cannot produce the required tree shape.

Verification level: no Level 1 fold test covers the mixed mark/dim fixture, and the current implementation would fail it. Add the design fixture as a fold test and change the processor to support nested mark/dim frames, or explicitly remove/defer that fixture from the design before calling the feature complete.

### High: HR attributes are folded but not rendered, and Level 2 only checks that the source text does not leak

The fold stores HR attributes under `darkmatter.hr` (`darkmatter/lib/src/markdown/render_tree/fold.rs:464`), matching the storage part of the design (`span-aware-processor-design.md:368`). The terminal renderer ignores those hints and always renders `HorizontalRule::new()` for every `ThematicBreak` (`biscuit-terminal/lib/src/render_tree/render.rs:424`); the browser renderer likewise lowers the node to a plain `<hr>` with generic node attrs (`renderable/src/tree/render/browser.rs:255`). The Level 2 test named `level2_tree_hr_attributes_render_styled_rule_in_real_terminal` only asserts surrounding paragraphs and absence of `style: waves` in plain text (`darkmatter/lib/tests/level2_render_tree_terminal.rs:346`), not a waves glyph/style/color/width.

Verification level: strongest useful verification is Level 1 for hint storage plus Level 2 for no raw-source leak. That is not sufficient for a user-observable HR-attribute rendering requirement. Either classify HR styling as an explicit internal-path renderer gap in the review/ledger and do not mark production-ready, or implement renderer consumption and add Level 2 assertions that distinguish styled HR output from a normal thematic break.

### High: benchmark baselines do not measure the span-aware tree path for the mark/dim/HR corpus

DMTR-6 requires a benchmark corpus including mark/dim/HR attributes (`spec.md:314`). The benchmark fixture comments say `mark_dim_hr` drives the span-aware fold (`darkmatter/lib/benches/migration_parity.rs:138`), but the tree side imports and calls only `fold_markdown_to_document` (`darkmatter/lib/benches/migration_parity.rs:38`, `darkmatter/lib/benches/migration_parity.rs:260`, `darkmatter/lib/benches/migration_parity.rs:311`, `darkmatter/lib/benches/migration_parity.rs:443`). That plain fold does not run `SpannedInlineStyleProcessor` / `SpannedRuleProcessor`, so the recorded `mark_dim_hr` tree numbers do not include the feature-specific processing they are meant to validate.

Verification level: benchmark evidence exists, but for the wrong implementation path. Use `fold_markdown_spanned_with_frontmatter` for fixtures that contain Darkmatter inline syntax, or split plain-vs-spanned groups and record baselines for the actual experimental entry-point path before relying on these numbers for readiness.

## Verification-Level Summary

| Requirement | Strongest observed verification | Assessment |
| --- | --- | --- |
| `==mark==` renders visibly in terminal | Level 2 WezTerm capture with reverse SGR assertion | Adequate for the simple mark case. |
| `⌄dim⌄` renders visibly in terminal | Level 2 WezTerm capture with dim SGR assertion | Adequate for the simple dim case. |
| Mixed mark/dim nesting | No matching test | Gap; implementation treats nested dim as literal while mark is open. |
| HR attributes affect rendered output | Level 1 hint storage; Level 2 no-leak check only | Gap; target renderers ignore the hints. |
| Benchmark evidence for mark/dim/HR path | Criterion baselines on plain fold | Gap; does not measure the span-aware implementation path. |

## Production Readiness

Not ready for production.

The simple mark and dim regressions from the prior review are improved, but the feature still misses a designed mixed-inline fixture, HR attributes are not user-observable through the tree renderers, and the published benchmarks do not exercise the span-aware path they claim to cover.

## Verification Performed

- Read `spec.md` and `span-aware-processor-design.md`.
- Reviewed `darkmatter::markdown::render_tree::{span, fold, entrypoints}`, parity tests, Level 2 terminal tests, render-tree terminal/browser lowering, and the Criterion benchmark harness.
- Attempted `cargo test -p darkmatter span_aware_fold_wraps_emphasis_inside_dim --color=never`; it was still compiling dependencies after about 60 seconds in this non-interactive review session, so I stopped it and do not claim test results.
- Requested `root` skill could not be used because no `root` skill is available in this session's skill catalog; I used the provided repo instructions plus the `renderable`, `rust`, and `rust-testing` skills.
