---
$schema: feature-review.yaml
ready: false
findings:
    - title: HTML block protection mistakes a CRLF split across raw nodes for a blank line
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-04T01:13:07-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: true
implemented_by: claude/opus
log: biscuit-terminal/features/2026-10-02-inline-prose/log.md
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-9.md
previous: 2026-10-02-inline-prose/review-8.md
next: 2026-10-02-inline-prose/review-10.md
---

# Inline Prose — Review 9

The feature is **not production ready**. Review 8's repair addresses all reported single-payload shapes, but misses line endings assembled from adjacent raw HTML nodes. A valid Windows CRLF becomes an apparent blank line when its CR and LF belong to different nodes. Strict rendering rejects valid content; warning rendering changes it unnecessarily, including breaking valid JavaScript.

This review evaluates the current working tree, including its existing uncommitted implementation changes. Only this review and the requested previous-review/spec metadata were changed permanently. Temporary public-API probes were removed. No implementation repair, commit, or lifecycle move was made.

## Previous review follow-through

Review 8 had one unblocked finding and no blocked findings. All earlier reviews also report no blocked code findings. The process-only human-review items in reviews 2–3 do not create an outstanding design decision or human-only activity.

| Review 8 finding | Result |
|---|---|
| HTML block protection rewrites opaque raw HTML | Implemented for its complete reported matrix. Raw payload ranges survive recursive browser serialization; payloads that fit remain byte-identical; actual raw blank lines are rejected under `Strict`, reported under `Warn`, and deliberately degraded under `Lossy`. Generated code retains blank-line protection. The retained six-test matrix passes. The sibling case of a CRLF crossing two raw ranges remains defective below. |

The implementation deliberately also reports raw blank lines in ordinary span text, quoted attributes, `pre`, and `textarea`, even where character references preserve HTML meaning. That conservative policy is documented and follows review 8's allowance for explicit unsupported/lossy handling. It is not a separate finding.

## Recurrence

This finding repeats review 8's **“HTML block protection rewrites opaque raw HTML”**: HTML-block protection re-encodes authored raw content unnecessarily because its decision does not account for the complete serialized input. Review 8's repair should have swept line endings assembled from adjacent raw nodes, in addition to line endings inside one raw payload, through every summary and column placement.

It also belongs to the broader output-preservation class of reviews 1–2's **“Markdown serialization changes literal backslashes and break meaning”** and review 6's **“Markdown break serialization changes meaning at boundaries and inside raw HTML.”** The earlier literal-text, parser-opacity, destination, soft-break whitespace, diagnostic migration, and terminal-capture cases remain covered by retained regressions; no additional recurrence in those sites was reproduced in the scoped checks.

The complete affected list is the direct disclosure summary; summaries inside strong, classed span, link, and unknown extension wrappers; the left and right columns; and direct raw block children of a column. Every site was tested at every split position. The clean browser and ordinary Markdown routes were tested with the same split-node inputs.

## Unblocked Findings

### High: HTML block protection mistakes a CRLF split across raw nodes for a blank line

**Defect class:** a serializer decides whether a line ending creates a blank line from individual source ranges rather than the concatenated stream, causing a valid CRLF to be rejected or rewritten when its bytes cross a node boundary.

In **renderable**, [the Markdown writer's `keep_html_block_open`](../../../renderable/src/tree/render/markdown.rs) combines CR and LF only when `raw_at(next) == owner`. That ownership check is appropriate for deciding which bytes can be changed, but it changes the interpretation of the line ending. Two adjacent raw ranges can hold the CR and LF of the same CRLF. HTML and Markdown input preprocessing read their concatenation as **one** line ending; the helper reads **two**, invents a blank line, and encodes the CR.

The public reproduction uses the retained disclosure fixture, dividing its raw child into two adjacent raw children:

```rust
let node = RenderNode::disclosure(
    vec![
        RenderNode::html("<span>a</span>\r", false),
        RenderNode::html("\n<span>b</span>", false),
    ],
    vec![RenderNode::paragraph(vec![RenderNode::text("body")])],
    None,
);
```

Calling **renderable**'s [public `render_markdown_node`](../../../renderable/src/tree/render/markdown.rs) with the MarkdownPlus dialect and `Strict` returns `RenderError::LossyRejected`, claiming the raw HTML holds a blank line. The equivalent single raw node succeeds. Neither input contains a blank line.

Under `Warn`, the split-node version emits:

```html
<details><summary><span>a</span>&#13;
<span>b</span></summary>
```

The inserted reference becomes a separate carriage return in HTML text, in addition to the physical newline. Splitting `<script>const a = 1;\r\nconsole.log(a);</script>` between CR and LF similarly inserts `&#13;` into JavaScript. An independent HTML reader extracts the reference literally; Node's syntax checker accepts the original script and rejects the rewritten script. Splitting `<span data-a=x\r\ndata-b=y>t</span>` at the same boundary changes `data-a` from `x` to `x` followed by a carriage return. The separate `data-b` attribute survives in this case.

This is reachable with two complete span payloads as shown above; the script and attribute cases additionally demonstrate that supported raw fragments can assemble an opaque or syntactic region. Browser rendering with raw HTML allowed preserves all three concatenations exactly.

The temporary public-result sweep exercised **2,416 cases**: nine payload shapes, every byte split including empty fragments, and eight placements. It found **24 mismatches**, exactly the three otherwise faithful CRLF payloads split between CR and LF in each of the eight placements. Other split positions, single LF, lone CR, actual blank lines with LF/CR/CRLF, and whitespace-only blank lines retained the expected strict success/rejection classification.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| MarkdownPlus disclosure summary, direct | Two complete spans, script, and unquoted attribute; every split position | CR/LF boundary fails under `Strict`; `Warn` inserts `&#13;` and reports loss | Same result as one raw node: keep a faithful CRLF byte-identical, with no lossy diagnostic. |
| Summary inside strong | Same nine-shape, every-split matrix | Same three boundary failures | Preserve stream-level CRLF semantics through recursive formatting. |
| Summary inside classed span | Same matrix | Same three boundary failures | Same as direct summary. |
| Summary inside link | Same matrix | Same three boundary failures | Same as direct summary; link wrapping must not change line-ending interpretation. |
| Summary inside unknown extension | Same matrix | Same three boundary failures | Retain the browser's child fallback and the raw bytes. |
| Columns, left paragraph | Same matrix | Same three boundary failures | Preserve raw CRLF in the left column. |
| Columns, right paragraph | Same matrix | Same three boundary failures | Preserve raw CRLF in the right column. |
| Columns, direct block HTML children | Same matrix | Same three boundary failures | Preserve raw CRLF across adjacent block payloads too. |
| All eight affected placements: clean split controls | All other split positions; LF and lone CR; actual LF/CR/CRLF blank lines; whitespace-only blank lines | Faithful inputs succeed; genuine raw blank lines are rejected | Retain these results while repairing split CRLF. |
| Ordinary raw output: concatenating root, paragraph, disclosure body, classed span, link label | The same three CRLF payloads at every split position, both dialects | Byte-identical output | Keep these clean routes free of HTML-block rewriting. A normal document root intentionally inserts block separators; this control uses its explicit concatenation mode. |
| Plain-Markdown summary and columns | Same three payloads at every split position | Byte-identical raw concatenation under the existing lossy plain-Markdown policy | Retain the documented plain-Markdown policy. |
| Browser renderer, all seven control placements | Same three payloads at every split position, raw HTML allowed | Byte-identical concatenation | Retain browser output and raw-range recording. |
| All eight HTML embedding placements, longer blank-line runs | One through eight line endings; LF, CR, CRLF; empty, space, tab, and mixed whitespace lines | Strict classification and independent CommonMark block-extent assertions pass | Genuine raw blank lines remain explicitly lossy; no premature container ending. |
| Generated text/code and raw/generated boundaries | Retained raw-payload and HTML-context matrices | Pass | Keep generated-code blank-line protection and prefer generated bytes when a real blank line needs protection. |

Recognize CRLF on the concatenated stream independently of range ownership. Retain ownership for each byte when deciding whether a genuine blank line can be protected without changing raw content. Do not simply combine and attribute a mixed-provenance ending to one node: that could make the repair rewrite a raw byte silently. Cover raw/raw, raw/generated, generated/raw, and generated/generated endings, including actual neighboring blank lines, through the same public-result matrix. The existing generated/raw controls pass but do not cover the new raw/raw case.

Retain the every-split regression across all eight affected placements and clean sibling routes. Update the helper's comments and the raw-content documentation if the repair changes the stated policy. Level 1 independent parsing and script compilation are appropriate for this serializer contract; terminal keyboard injection and human approval are unnecessary.

## Blocked Findings

None. Preserving a faithful raw payload and interpreting CRLF consistently are existing contracts.

## Input Robustness Matrix

The feature's new load-bearing configuration field remains JSON `browser.block_element`. No additional file/configuration reader was introduced by the latest repair. The retained matrix derives each cell from the same production-serialized fixture and asserts the browser result or public deserialization rejection; it passes in the renderable suite.

| Shape | JSON `browser.block_element`: expected and observed |
|---|---|
| Positive control | All seven lowercase tag variants render their element and preserve the fixture's other attributes. |
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
| Renderable `just test`, before temporary probes | 624 passed, including the six new raw-HTML tests and retained literal-text, break, HTML-context, serialization, validation, and streaming tests. |
| Biscuit-terminal `just test prose_` | 228 passed. |
| Biscuit-terminal `just test two_column_markdown_html` | Three passed. |
| Darkmatter `just test code_link` | 16 passed, including templates and completion. |
| Biscuit-terminal `just test-l2 inline_code` | Four representative scenarios executed in tmux/WezTerm and passed. Two Kitty tests exited through availability gates; those exits do not prove Kitty rendering. The library portion selected zero tests and is not credited as evidence. |
| Root `just check-tier-coverage biscuit-terminal` and `renderable` | Zero stranded tests. |
| Renderable and biscuit-terminal `just lint` | Passed. |
| Temporary every-split public-API sweep | Failed as expected: 24 mismatches across 2,416 cases, all enumerated above. |
| Temporary longer-run/control sweep | Passed: 798 cases, including independent CommonMark HTML-block extent checks. |
| Temporary split-node clean-route sweep | Passed: 2,128 browser/Markdown output assertions. |
| Independent HTML/script checks | Confirmed the attribute-value change and JavaScript syntax failure. The original JavaScript passes syntax checking; no script was executed. |

Three probe setup errors were corrected before collecting the stated evidence: the initial root recipe interpreted a test filter as a package name, the browser control initially used the default raw-HTML escaping policy rather than `Allow`, and the root control initially used document block separators rather than explicit concatenation. None is counted as an implementation defect. All temporary test files were removed afterward.

Renderable's added integration tests are automatically declared targets. Biscuit-terminal's earlier prose/container tests are declared in its consolidated Level 1 target, and the relevant terminal scenarios are in its feature-enabled Level 2 target with a live recipe. No permanent test was added or renamed in this review.

The terminal recipe owned background pane setup and cleanup; no foreground activation was requested. Full downstream suites, browser-resource tests, auxiliary-target compilation, and cross-OS runs were not repeated. Previously recorded unrelated failing guards and downstream tests are not new findings or readiness gates.

## Requirement-to-verification mapping

The criterion numbers refer to the spec's Acceptance Criteria; each row describes the actual behavior.

| User-facing requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Inline/block shape, wrapper-free output, empty content, portable links (1, 2, 7, 11, 25) | Level 1 public component/tree/HTML/Markdown tests; representative Level 2 link/break capture | Retained checks pass. |
| Paragraphs, newline modes, whitespace, escaping, normalized line endings (3–7, 26–27) | Level 1 grammar and independent-reader matrices; Level 2 displayed geometry | Prose input checks pass. The ancillary shared-renderer split-CRLF defect is above. |
| Paragraph HTML tags, streaming parity, serialization, validation (8, 31) | Level 1 emitted structure and production-fixture robustness matrix | Appropriate level; pass. |
| Fenced/code-span opacity, style scope, inline fence degradation, safe Markdown spelling (9, 14, 16, 18–19, 27–28, 30) | Level 1 component/read-back matrices; Level 2 container capture | Retained typed-code checks pass. Raw HTML assembled across nodes remains defective. |
| Layout exactly once and structural container embedding (10, 12, 31) | Level 1 public container/CLI tests; existing Level 2 geometry checks | Targeted column tests and terminal scenarios pass. |
| Migrated diagnostics and literal code values (13, 17, 20) | Retained Level 1 diagnostic/snapshot tests; Level 2 status/container capture | Prior repairs remain present; full consumer suites were not repeated. |
| Dim code, enclosing style restoration, hyperlink extent, unstyled fallback (15, 21, 29) | Level 1 capabilities and Level 2 captured terminal cells/styles/links | Representative tmux/WezTerm cases pass. |
| Code-link labels, destination parity, templates, catalog/completion (22–23) | Level 1 public composition/rendering and completion tests | Targeted suite passes. |
| Tests/lint and auxiliary/consumer compilation (24) | Current scoped tests/lint and earlier implementation records | No new evidence claimed for unrerun downstream or auxiliary targets. |

No feature requirement handles keyboard or mouse events, so Level 3 is unnecessary. No additional verification-level mismatch was identified. Cross-OS evidence and human sign-off remain external to this readiness decision.
