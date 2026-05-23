---
ready: false
agent: codex
model: ""
addressed_in: review-8 (this iteration's fix)
---

## Resolution (Iteration 8)

Both High findings have been implemented in the span-aware processor chain:

1. **Escaped mark provenance (Finding 1).** `SpannedInlineStyleProcessor` now
   accepts the original source string and, when a `==` delimiter sits at the
   start of a text event whose preceding source byte is `\`, marks the
   delimiter as escaped via a new `cross_event_escape` flag. The reverted
   literal `==` text event is emitted with its byte range extended back one
   byte to cover the `\` that `pulldown-cmark` consumed as a CommonMark escape.
   The span-tier test
   `escaped_mark_delimiter_literal_covers_backslash_and_delimiter_bytes`
   (renamed) now pins `4..7` exactly. A new fold-tier counterpart
   `span_aware_fold_escaped_mark_literal_text_includes_backslash_byte` asserts
   the literal's `SourceLocation.bytes` at the assembled `RenderNode` layer.

2. **Generated HR provenance (Finding 2).** `SpannedRuleProcessor::close_paragraph`
   now derives the generated event's `range` and `source` from the buffered
   text event (`self.buffer[0].range`) instead of from
   `Start(Paragraph)..End(Paragraph)`. That removes the trailing-newline byte
   `pulldown-cmark` includes in its `End(Paragraph)` range. The span-tier test
   `generated_hr_event_source_covers_paragraph_body_bytes` now asserts the
   exact `0..body.len()` range; the fold-tier test
   `span_aware_fold_hr_source_location_pins_paragraph_body_bytes` does the
   same at the `RenderNode.span.location` layer.

Other behavior is unchanged. Module-level *Range Policy* docs in
`darkmatter/lib/src/markdown/render_tree/span.rs` were updated to describe
both behaviors precisely.

---


# Review: Darkmatter Tree Rendering Migration, Iteration 7

## Findings

### High: escaped mark source spans intentionally violate the span-aware design

The spec makes `span-aware-processor-design.md` the implementation contract for exact byte ranges, including escaped delimiters (`spec.md:153`-`156`). That design says escaped delimiters become literal text and their source span includes the escape prefix (`span-aware-processor-design.md:125`-`134`), and the escaped mark fixture specifically requires the literal `==` span to include the original backslash byte (`span-aware-processor-design.md:281`-`291`).

The implementation now pins the opposite behavior for `\==`: the new test says the literal `==` only spans `5..7`, explicitly excluding the backslash at byte 4 (`darkmatter/lib/src/markdown/render_tree/span.rs:1008`-`1055`). The test comment explains why this happens with the current pulldown-cmark chain, but that is still a design/implementation mismatch, not closure of the requirement. The fold-level test only asserts that no mark span is opened and does not assert the escaped literal's `SourceLocation` (`darkmatter/lib/src/markdown/render_tree/fold.rs:1480`-`1490`).

This matters because DMTR-3 is about preserving provenance for diagnostics and tooling. A diagnostic attached to the literal escaped delimiter would now point at only `==`, while the design requires it to point at the source bytes that produced the literal, `\==`.

Verification level: Level 1 tests exist, but they encode a weaker contract than the design. Fix either the implementation so the escaped mark event/fold span covers `\==`, or update the design/spec to explicitly accept this pulldown-cmark limitation before marking the feature ready.

### High: generated HR provenance tests allow a range broader than the design's exact fixture

The generated-HR design says the synthetic event points back to the original paragraph bytes (`span-aware-processor-design.md:155`-`170`), and the concrete fixture expects `--- { style: waves }` to become a generated `ThematicBreak` at `location=0..20` (`span-aware-processor-design.md:367`-`375`).

The span-aware rule processor builds the generated HR source range from `start_range.start..end_event.range.end` (`darkmatter/lib/src/markdown/render_tree/span.rs:669`-`684`), and the tests deliberately accept any end between `body.len()` and `input.len()` because pulldown's paragraph end may include the trailing newline (`span.rs:1098`-`1144`, `fold.rs:1535`-`1578`). That means a `0..21` span for `"--- { style: waves }\n"` passes even though the design fixture pins the source paragraph itself at `0..20`.

For diagnostics/source mapping, including the trailing newline is a precision regression. The processor already has the buffered text event range available; use that to pin the generated HR source to the actual paragraph body/attribute bytes, and add exact Level 1 assertions for both the spanned event and the folded `ThematicBreak`.

Verification level: Level 1 tests exist, but they assert containment instead of the exact byte-range policy. This is insufficient for the provenance requirement DMTR-3 was introduced to satisfy.

## Verification-Level Summary

| Requirement | Strongest observed verification | Assessment |
| --- | --- | --- |
| Experimental tree entry points for Markdown, MarkdownPlus, Browser, and Terminal | Level 1 smoke and adapter tests | Adequate for internal entry points. |
| Mark/dim/HR constructs reach tree Browser and Terminal paths | Level 1 entry-point tests plus optional Level 2 WezTerm tests for terminal styling | Adequate for visible terminal behavior when Level 2 is required in CI. |
| Raw HTML defaults to escaping through `render_tree_html` | Level 1 entry-point tests for block and inline raw HTML | Prior review-6 gap appears closed. |
| Split mark and dim byte ranges | Level 1 exact event and fold tests | Adequate for covered non-escaped fixtures. |
| Escaped mark delimiter provenance | Level 1 test exists but asserts the opposite of the design | Gap. |
| Generated HR source range | Level 1 tests exist but permit a broader newline-inclusive span | Gap. |

## Production Readiness

Not ready for production.

Iteration 7 closes the raw-HTML entry-point coverage gap and adds much stronger span tests, but two provenance requirements are still not implemented as designed. Both are in the core DMTR-3 source-span contract.

## Verification Performed

- Read `spec.md`, `span-aware-processor-design.md`, and `review-6.md`.
- Reviewed `darkmatter::markdown::render_tree::{entrypoints, span, fold, code_renderer}`, `darkmatter/lib/tests/render_tree_parity.rs`, `darkmatter/lib/tests/level2_render_tree_terminal.rs`, and `darkmatter/lib/benches/migration_parity.rs`.
- Attempted a targeted `cargo test -p darkmatter markdown::render_tree::span::tests::escaped_mark_delimiter_literal_covers_delimiter_bytes --color=never`; Cargo was still cold-compiling after about 65 seconds, so I stopped it and do not claim test results.
- The requested `root` skill is unavailable in this session's skill catalog; I used the provided repo instructions and the `renderable` skill.
