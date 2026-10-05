---
$schema: feature-review.yaml
ready: false
findings:
    - title: Markdown table escaping misses raw HTML, span classes, and footnote identifiers
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-04T02:00:30-07:00
spec: 2026-10-02-inline-prose/spec.md
log: biscuit-terminal/features/2026-10-02-inline-prose/log.md
implemented: true
implemented_by: claude/opus
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-10.md
previous: 2026-10-02-inline-prose/review-9.md
next: 2026-10-02-inline-prose/review-11.md
---

# Inline Prose — Review 10

The feature is **not production ready**. Review 9's reported CRLF defect is repaired. One remaining defect class has three sites: raw HTML, generated span classes, and footnote identifiers bypass Markdown table escaping. A literal pipe can prevent a header from becoming a table or silently shift and discard body-cell content.

This review evaluates the current working tree, including existing uncommitted implementation changes. Only this review and the requested previous-review/spec metadata were changed permanently. Temporary public-API probes were removed. No implementation repair, commit, or lifecycle move was made.

## Previous review follow-through

Review 9 had one unblocked finding and no blocked findings. Reviews 1–8 also have no outstanding blocked code findings. The process-only human-review items in reviews 2–3 do not require a new design decision or human-only test.

| Previous finding | Result |
|---|---|
| HTML block protection mistakes a CRLF split across raw nodes for a blank line | Implemented. The retained every-split matrix passes across all eight HTML embedding placements and clean sibling routes. The repair recognizes stream-level CRLF while retaining ownership for each byte. |
| Required raw/generated boundary sweep | Implemented. Raw/raw, raw/generated, generated/raw, and generated/generated controls, including neighboring actual blank lines, pass. Generated CR retains its documented character-reference policy. |
| Additional line-ending siblings discovered during repair | Container prefixing, continuation indentation, code-fence sizing, heading CRLF, and raw HTML line endings in headings/cells have retained passing public-result tests. |

The implementation log explicitly deferred the suspected raw-HTML table-pipe problem without reproducing it. It is reproduced below, together with two sibling escape bypasses. No human decision is needed to preserve table structure or report an unrepresentable value.

## Recurrence

This finding repeats the context-dependent serialization class of review 7's **“HTML-backed Markdown output does not lower its children for the enclosing context”** and review 4's **“Markdown serialization reinterprets literal text and entity spellings.”** Reviews 1–2's **“Markdown serialization changes literal backslashes and break meaning”**, review 5's literal-encoding finding, review 6's break-serialization finding, and reviews 8–9's raw-content preservation findings are related members of the same broader output-preservation class.

Those repairs should have swept every source of authored characters inside the table writer: raw HTML payloads, generated HTML attributes, footnote identifiers, text, code, link labels/destinations/titles, and image alternative text/destinations/titles. The latest repair swept line endings in table cells but explicitly left pipes behind. This review carries the complete affected writer-site list: raw HTML output, generated span class attributes, and footnote reference identifiers. The text/code/link/image siblings are clean in the pipe checks.

Comparison with every earlier review found no newly reproduced defect in the earlier diagnostic migration, code opacity, wrapper scanning, portable destinations, soft-break trimming, terminal-capture framing, serialized-field validation, or background-tag documentation classes. The relevant retained tests pass; this statement does not claim that every consumer suite was rerun.

## Unblocked Findings

### High: Markdown table escaping misses raw HTML, span classes, and footnote identifiers

**Defect class:** a serializer protects selected text fields but emits other authored fields directly into a delimiter-based container, so the reader interprets their literal delimiters as container structure.

In **renderable**, [the shared Markdown writer](../../../renderable/src/tree/render/markdown.rs) activates table-cell escaping for descendants, but three branches bypass pipe protection:

- `render_html`, which emits deliberately raw HTML, checks line endings but returns pipe-containing payloads unchanged.
- `render_span`, which emits an inline HTML span in MarkdownPlus, inserts joined class names directly into its attribute.
- The `NodeKind::FootnoteReference` branch emits its identifier directly between footnote delimiters.

GFM determines table cells before parsing inline HTML, code, or footnotes. A pipe remains a cell delimiter even inside a quoted HTML attribute. These bypasses therefore matter to table structure, not just to a source-formatting preference.

The reproduction copies the retained two-column fixture in [the single-line raw-HTML tests](../../../renderable/tests/markdown_single_line_raw_html.rs), replacing one cell with `RenderNode::html("<span>a|b</span>", false)`. Calling renderable's public `render_markdown_node` with MarkdownPlus and `Strict` succeeds without diagnostics and emits:

```markdown
| H1 | H2 |
| --- | --- |
| <span>a|b</span> | z |
```

An independent GFM reader returns body cells `<span>a` and `b</span>`; the intended second-cell value `z` is discarded. Putting the same payload in the header produces no table at all because the header and delimiter row have different cell counts. A generated class `a|b` and a footnote reference identifier `a|b` cause the same failures. The footnote control includes its definition and proves that the identifier resolves normally outside a table; this is a valid identifier reaching the wrong encoding context.

The raw-HTML sweep exercised **18,800 cases**: ten payload shapes, every byte split including empty fragments, ten wrapper routes, header/body placement, either column, and both dialects. It found **13,040 failures** in **560 combinations** of payload, wrapper, row position, column, and dialect. The seven unescaped-pipe shapes failed at every split. The preescaped-pipe and two character-reference controls preserved table structure. The sweep checked both cell counts and retained cell content: counting cells alone misses body-row truncation.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Raw HTML in table header, either column | Span text, quoted/unquoted attribute, comment, script, style, and code element containing an unescaped pipe; every byte split | Header no longer parses as a table | Keep the authored header and table structure, or explicitly reject an unrepresentable payload. |
| Raw HTML in table body, either column | Same matrix | Content splits; later content is shifted or discarded | Keep both cell values in their intended columns. |
| Raw HTML through strong, emphasis, delete, link, neutral span, classed span, unknown extension, mark, and dim | Same matrix, header/body and either column | Same failures in both dialects | Apply the same cell policy recursively, including fragmented raw nodes. |
| Raw HTML controls | Preescaped pipe; numeric pipe reference in text and quoted attribute | Table structure survives at every split | Retain the clean representations; do not double-escape them. Structure success alone does not establish arbitrary raw-source fidelity. |
| Generated span class attribute | Class `a|b`, direct header/body | MarkdownPlus splits the cell with no diagnostic; plain Markdown removes the class under its existing degradation policy and keeps the cells | Use a table-safe attribute spelling in MarkdownPlus; retain plain-Markdown degradation. |
| Footnote reference identifier | `a|b`, header/body, both dialects, with a matching definition | Cell splits; `Strict` and `Warn` accept it without diagnostics | Preserve the reference and its identifier without splitting the row, or report a representation limitation. |
| Ordinary text | `a|b` | Cell value remains `a|b` | Retain existing escaping. |
| Inline code | `a|b` and a literal backslash before the pipe | Independent reader retains the exact code value in one cell | Retain code fencing and backslash preservation. |
| Link label, destination, title | Pipe in each field | Table structure survives; emitted destination/title use cell escapes | Retain existing field-specific escaping. |
| Image alternative text, destination, title | Pipe in each field | Table structure survives; fields use cell escapes | Retain existing field-specific escaping. |
| Heading and section-heading controls, including strong/link/classed-span wrapping | Same ten raw payload shapes at every split, both dialects | Heading structure survives | Pipes outside table rows must retain their ordinary meaning. |
| HTML-block summaries/columns, ordinary raw output, browser output | Writer routes inspected; retained raw-payload and HTML-context matrices rerun | Do not use a pipe-delimited row; retained checks pass | Limit cell protection to table contexts. |

The typed-field probe ran 44 header/body cases. A separate 18-case strictness sweep confirmed that MarkdownPlus raw HTML/classes and both-dialect footnote identifiers silently succeed under `Strict`, `Warn`, and `Lossy`. Plain Markdown rejects raw HTML/classes under `Strict` for its existing portability policy; its `Warn` raw-HTML output still breaks the table. That generic portability warning does not repair cell structure.

Make every authored field emitted inside a cell table-safe. Preserve the distinction between raw HTML, generated attributes, code contents, and footnote syntax. A final global string replacement risks double-escaping the already-correct text/code/link/image paths. For raw HTML, account for GFM's removal of pipe escapes before inline parsing; do not blindly substitute character references inside scripts, comments, or other opaque regions. If a faithful representation is unavailable, use the established strict/warn/lossy handling. The repair must cover header and body cells, either column, all listed wrappers, split fragments, and the clean controls.

Retain public-result regressions that assert the independent reader's structure **and cell values**, including the neighboring sentinel value. Add typed class and resolved-footnote cases to that matrix. Update the affected writer comments and [renderable's table/raw-HTML documentation](../../../renderable/docs/tree-rendering.md). This is a Level 1 serialization contract; keyboard injection and human approval are unnecessary.

The raw-HTML and footnote bypasses predate the latest repair, while the same table writer was changed during this feature. They are shared-renderer defects rather than a new InlineProse grammar requirement. The generated-class route is also used by the feature's styled inline content. All three belong in the same repair to avoid another missed-sibling cycle.

## Blocked Findings

None.

## Input Robustness Matrix

The feature's only newly introduced load-bearing serialized configuration field remains JSON `browser.block_element`. No new format/configuration reader was added by the latest repair. The retained test starts from a production-serialized paragraph fixture, edits one field, and asserts browser output or public deserialization rejection. It passes in the full renderable suite.

| Shape | JSON `browser.block_element`: expected and observed |
|---|---|
| Positive control | All seven lowercase tag variants render their element and retain the fixture's other attributes. |
| Absent | Legacy paragraph renders `<p>`. |
| Explicit null | Rejected. |
| Wrong whole-field type | Number rejected. |
| Wrong type, one element | Mixed array rejected for this scalar field. |
| Wrong type, every element | All-invalid array rejected. |
| Empty | Empty string, array, and object rejected. |
| Duplicate key | Rejected. |
| Trailing or invalid content | Rejected. |
| Invalid spelling | Unknown and uppercase tag names rejected. |

## Verification

| Check | Result |
|---|---|
| Renderable `just test`, before temporary probes | 643 passed, including every-split CRLF, raw/generated provenance, container line endings, single-line raw HTML, literal/break serialization, HTML-context, paragraph-tag, and streaming tests. |
| Biscuit-terminal `just test prose_` | 228 passed. |
| Biscuit-terminal `just test two_column_markdown_html` | Three passed. |
| Darkmatter `just test code_link` | 16 passed, including templates and completion. |
| Biscuit-terminal `just test-l2 inline_code` | Three representative tmux/WezTerm scenarios passed. The WezTerm prose scenario failed when Bash emitted `child setpgid: Operation not permitted` inside the output capture; the prose row itself was correct. Two Kitty tests exited through availability gates and supply no Kitty evidence. The library selected zero tests. |
| Isolated `just test-l2 prose_inline_code_and_breaks_in_wezterm` | Passed with real WezTerm capture. This resolves the immediate assertion failure but does not prove the host shell error cannot recur. |
| Root tier coverage checks for renderable and biscuit-terminal | Zero stranded tests. |
| Renderable and biscuit-terminal `just lint`, after probe removal | Passed. |
| Temporary raw-HTML table sweep | 18,800 cases; 13,040 independent-reader structure/content failures, enumerated above. |
| Temporary typed-field and strictness controls | 44 typed-field cases and 18 strictness cases; confirmed all three bypasses and clean text/code/link/image routes. A separate footnote control resolves `a|b` outside a table. |

Initial probe setup mistakes used the root recipe with a test filter and a duplicated relative directory. Those commands supplied no evidence and were corrected before the stated runs. A lint run while the temporary probe existed rejected probe-only lint issues; the clean-source rerun passed. No implementation failure is inferred from those setup errors.

Renderable automatically discovers its integration targets. The new retained test files are compiled and selected by Level 1. Biscuit-terminal's prose/container tests are declared in its consolidated Level 1 target, and terminal scenarios are in its feature-enabled Level 2 target with a live recipe. No permanent test was added or renamed during this review. Terminal recipes owned pane setup and cleanup; no foreground activation was requested.

Full downstream suites, browser-resource tests, auxiliary-target compilation, and cross-OS runs were not repeated. Previously recorded unrelated failures are not new findings. Cross-OS evidence and human sign-off are external to this readiness decision.

## Requirement-to-verification mapping

Numbers refer to the spec's Acceptance Criteria; the descriptions state the behavior being verified.

| Requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Inline/block shape, wrapper-free output, empty inputs, links (1–2, 7, 11, 25) | Level 1 public tree/HTML/Markdown checks; representative Level 2 capture | Scoped checks pass. |
| Paragraphs, break modes, whitespace, normalized line endings, style scope (3–7, 26–27) | Level 1 grammar and independent-reader matrices; Level 2 displayed geometry | Pass, including review 9's repaired CRLF cases. |
| Paragraph HTML tags, validation, serialization, streaming parity (8, 31) | Level 1 public-result fixture matrix | Appropriate level; pass. |
| Code opacity, fenced-code shape, safe Markdown fences, inline normalization (9, 14, 16, 18–19, 27–28, 30) | Level 1 projection/read-back checks; Level 2 container capture | Typed code checks pass. Ancillary table-field escape bypasses are reported above. |
| Layout exactly once and structural container embedding (10, 12, 31) | Level 1 public container/CLI checks; representative Level 2 geometry | Scoped checks pass. |
| Migrated diagnostics and literal code values (13, 17, 20) | Retained Level 1 regressions/snapshots; representative Level 2 container capture | Relevant repairs remain present; full consumer suites were not rerun. |
| Dim code, style restoration, hyperlink extent, unstyled fallback (15, 21, 29) | Level 1 capabilities and Level 2 terminal cell/style/link capture | tmux/WezTerm evidence passes, with the initial host-shell failure disclosed above. |
| Code-link labels, destinations, templates, catalog/completion (22–23) | Level 1 public composition/rendering and completion tests | Targeted suite passes. |
| Package tests/lint and auxiliary/consumer compilation (24) | Current scoped checks and prior implementation records | No new evidence claimed for unrerun suites or targets. |

No requirement handles keyboard or mouse input, so Level 3 is unnecessary. No additional verification-level mismatch was identified.
