---
ready: false
agent: codex
model: ""
---

# Review: Darkmatter Tree Rendering Migration, Iteration 6

## Findings

### High: raw-HTML escape default is implemented but not verified through the Darkmatter tree HTML entry point

DMTR-9 requires the experimental Browser/HTML tree path to preserve the safe legacy baseline by defaulting raw Markdown HTML to escaping. The adapter does set `BrowserRenderOptions.raw_html = RawHtmlPolicy::Escape` in `browser_options_from_html_options` (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:153`), but the entry-point tests only smoke-render a heading fixture (`entrypoints.rs:237`) and never render raw HTML through `render_tree_html`. The parity helper deliberately overrides the tree side to `RawHtmlPolicy::Allow` (`darkmatter/lib/tests/render_tree_parity.rs:269`) so raw HTML can be compared against the legacy renderer, which means the parity raw-HTML test does not exercise the production safety default (`render_tree_parity.rs:596`).

This leaves a user-observable security/safety requirement unpinned at the adapter boundary. A future change could accidentally switch the entry point to `Allow` while the lower-level renderable tests still pass, because those tests only prove the browser renderer can escape when asked. Add a Level 1 integration/unit test that calls `render_tree_html(&md, &HtmlOptions::default())` with block and inline raw HTML, asserts the output contains escaped text such as `&lt;div`, and asserts it does not contain a live `<div>` / `<script>` node from source. Keep the existing `RawHtmlPolicy::Allow` parity test as a separate accepted-divergence fixture.

Verification level: strongest observed coverage for the Darkmatter entry point is code inspection; renderable has Level 1 coverage for the lower-level renderer, but not for the Darkmatter option adapter. Browser HTML output is user-observable, and DMTR-9 is an explicit cutover gate, so this is not ready.

### High: span-aware processor tests do not assert the exact byte-range policy from the design

The span-aware design is specifically about preserving source ranges: split mark text must get exact byte subranges, Unicode dim delimiters must use UTF-8 byte offsets, escaped delimiters must include the escape byte in the literal's range, and generated HRs must point back to the paragraph source range. The implementation comments document those policies, but the tests only count that mark/dim start/end events exist (`darkmatter/lib/src/markdown/render_tree/span.rs:799`, `span.rs:830`) and that one HR event is generated (`span.rs:863`). The fold tests check only that some source location exists for HR attributes, not that it is the specified range.

This is a coverage gap against the core DMTR-3 contract. A bug that shifted child spans to whole-text ranges, dropped the backslash from escaped delimiter spans, or used character offsets for `⌄` would still pass the current tests. Add Level 1 assertions matching `span-aware-processor-design.md`: `plain ==highlighted== after` should produce mark opener `6..8`, child text `8..19`, closer `19..21`, and container span `6..21`; `normal ⌄dimmed⌄ after` should pin the three-byte delimiter ranges; escaped `\\==` and `\\⌄` should emit literal text with ranges that include the backslash; `--- { style: waves }` should produce a generated thematic break whose `SourceLocation.bytes` covers the full source paragraph.

Verification level: Level 1 tests exist for the presence of mark/dim/HR constructs, but not for the designed provenance/range behavior. Since source provenance is the reason this new processor exists and will drive diagnostics/tooling, the feature should not be marked production-ready until these exact range fixtures are pinned.

## Verification-Level Summary

| Requirement | Strongest observed verification | Assessment |
| --- | --- | --- |
| Experimental tree entry points for Markdown, MarkdownPlus, Browser, and Terminal | Level 1 smoke tests | Adequate. |
| `TerminalOptions::color_depth = None` reaches the tree terminal context | Level 1 adapter and rendered-byte tests; benchmark helper assertion | Prior review-5 gap appears closed. |
| `migration/terminal_no_color` measures a no-color tree context | Benchmark helper now sets and asserts `TerminalColorDepth::None` | Prior review-5 gap appears closed. |
| Mark/dim/HR constructs reach terminal and browser entry points | Level 1 entry-point tests plus optional Level 2 WezTerm tests | Adequate for visible terminal/browser behavior when Level 2 is enabled. |
| Raw HTML defaults to safe escaping through `render_tree_html` | No direct Darkmatter entry-point test; parity uses `RawHtmlPolicy::Allow` | Gap. |
| Span-aware processor preserves exact byte ranges and generated provenance | Presence-only Level 1 tests; no exact range assertions | Gap. |

## Production Readiness

Not ready for production.

Iteration 6 appears to fix the no-color adapter/benchmark issue from iteration 5, but the remaining gaps are both acceptance-level verification holes. The raw HTML default is a security-relevant Browser/HTML behavior, and the exact span/range policy is the core technical purpose of the span-aware processor.

## Verification Performed

- Read `spec.md`, `span-aware-processor-design.md`, and the previous `review-5.md`.
- Reviewed `darkmatter::markdown::render_tree::{entrypoints, fold, span}`, `darkmatter/lib/tests/render_tree_parity.rs`, `darkmatter/lib/tests/level2_render_tree_terminal.rs`, `darkmatter/lib/benches/migration_parity.rs`, and the legacy `RuleProcessor`.
- Attempted targeted `cargo test` runs, but the commands spent the non-interactive budget waiting on Cargo/package locks and cold compilation. I stopped the spawned cargo processes and do not claim test results.
- The requested `root` skill is unavailable in this session's skill catalog; I used the provided repo instructions and the `renderable` skill.
